//! Automatic backups: an encrypted `.nexus` copy every N hours while the vault is
//! unlocked, keeping the newest `keep` files. Only files this module wrote
//! (`auto-<vault>-<id>-<timestamp>.nexus`) are ever rotated away. When nothing in
//! the vault changed since the last copy, no new file is written: ten identical copies
//! would only push older, different ones out of rotation and fill the disk.

use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::backup::{EXTENSION, hex, write_backup};
use crate::core::error::{AppError, AppResult};
use crate::core::time::civil_utc;
use crate::crypto::aead::SecretKey;
use crate::database::Database;
use crate::database::repo;

/// Plaintext `settings` key with the time of the last automatic copy (per vault).
const LAST_KEY: &str = "last_auto_backup_at";
/// When the vault was last checked, whether or not that produced a copy.
const CHECKED_KEY: &str = "last_auto_backup_check_at";
/// Fingerprint of the vault contents at the last automatic copy.
const FINGERPRINT_KEY: &str = "last_auto_backup_fingerprint";

/// Changes whenever anything a backup holds changes, settings aside. Every write of an
/// encrypted row draws a fresh nonce, so nonces plus the plaintext flags are enough;
/// nothing is decrypted. Settings are left out: they change on their own (this very
/// timestamp) and are not worth a copy.
fn fingerprint(db: &Database) -> AppResult<String> {
    let mut hasher = Sha256::new();
    for sql in [
        "SELECT id || ':' || hex(nonce) || ':' || IFNULL(folder_id, '') || ':' || is_favorite
             || ':' || IFNULL(deleted_at, '') FROM items ORDER BY id",
        "SELECT id || ':' || hex(nonce) FROM folders ORDER BY id",
        "SELECT id || ':' || hex(nonce) FROM attachments ORDER BY id",
        "SELECT id || ':' || hex(nonce) FROM key_envelopes ORDER BY id",
        "SELECT hex(kdf_salt) || ':' || kdf_memory_kib || ':' || kdf_iterations FROM vault_meta",
    ] {
        let mut stmt = db.conn().prepare(sql)?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        for row in rows {
            hasher.update(row?.as_bytes());
            hasher.update(b"|");
        }
        hasher.update(b"#");
    }
    Ok(hex::encode(&hasher.finalize()))
}

fn setting(db: &Database, key: &str) -> Option<String> {
    repo::load_settings(db.conn())
        .ok()?
        .into_iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v)
}

/// `auto-<file stem>-<8 chars of the registry id>-` — unique per vault, so two vaults
/// sharing a backup folder never rotate each other's copies.
pub fn file_prefix(vault_path: &Path, vault_id: &str) -> String {
    let stem: String = vault_path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "vault".into())
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let id: String = vault_id
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .take(8)
        .collect();
    format!("auto-{stem}-{id}-")
}

fn file_name(prefix: &str, millis: i64) -> String {
    let (y, mo, d, h, mi, s) = civil_utc(millis);
    format!("{prefix}{y:04}-{mo:02}-{d:02}_{h:02}-{mi:02}-{s:02}.{EXTENSION}")
}

/// Automatic copies of one vault in `folder`, oldest first (names sort by time).
pub fn list(folder: &Path, prefix: &str) -> Vec<PathBuf> {
    let suffix = format!(".{EXTENSION}");
    let mut files: Vec<PathBuf> = fs::read_dir(folder)
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.is_file()
                        && p.file_name()
                            .and_then(|n| n.to_str())
                            .is_some_and(|n| n.starts_with(prefix) && n.ends_with(&suffix))
                })
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

/// Deletes the oldest automatic copies beyond `keep`. Returns how many were removed.
pub fn rotate(folder: &Path, prefix: &str, keep: usize) -> usize {
    let files = list(folder, prefix);
    let excess = files.len().saturating_sub(keep);
    files
        .iter()
        .take(excess)
        .filter(|p| fs::remove_file(p).is_ok())
        .count()
}

/// Time of the last automatic copy actually written.
pub fn last_backup_at(db: &Database) -> Option<i64> {
    setting(db, LAST_KEY).and_then(|v| v.parse().ok())
}

/// Time of the last check (a copy, or "nothing changed"); the schedule counts from it.
pub fn last_checked_at(db: &Database) -> Option<i64> {
    setting(db, CHECKED_KEY)
        .and_then(|v| v.parse().ok())
        .or_else(|| last_backup_at(db))
}

/// Writes one automatic copy, records its time and rotates old ones. Unless `force`d,
/// an unchanged vault whose last copy is still there only gets the check time recorded
/// (so the next check waits a full interval) and `None` is returned.
pub fn run(
    folder: &Path,
    prefix: &str,
    keep: usize,
    db: &mut Database,
    key: &SecretKey,
    now: i64,
    force: bool,
) -> AppResult<Option<PathBuf>> {
    let current = fingerprint(db)?;
    let unchanged = setting(db, FINGERPRINT_KEY).as_deref() == Some(current.as_str())
        && !list(folder, prefix).is_empty();
    if unchanged && !force {
        db.transaction(|tx| repo::set_setting(tx, CHECKED_KEY, &now.to_string()))?;
        return Ok(None);
    }

    fs::create_dir_all(folder)?;
    let path = folder.join(file_name(prefix, now));
    if path.exists() {
        // two runs within one second: the existing copy is just as fresh
        return Ok(Some(path));
    }
    write_backup(&path, db, key)?;
    db.transaction(|tx| {
        repo::set_setting(tx, LAST_KEY, &now.to_string())?;
        repo::set_setting(tx, CHECKED_KEY, &now.to_string())?;
        repo::set_setting(tx, FINGERPRINT_KEY, &current)
    })?;
    rotate(folder, prefix, keep.max(1));
    Ok(Some(path))
}

pub fn is_due(last: Option<i64>, interval_hours: u32, now: i64) -> bool {
    last.is_none_or(|last| now - last >= i64::from(interval_hours) * 3_600_000 || now < last)
}

/// Rejects relative or obviously wrong folders before they are saved in settings.
pub fn validate_folder(folder: &str) -> AppResult<PathBuf> {
    let path = PathBuf::from(folder);
    if !path.is_absolute() {
        return Err(AppError::Validation("auto_backup_dir"));
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::vault;
    use crate::crypto::kdf::KdfParams;

    #[test]
    fn writes_records_and_rotates_only_its_own_files() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::open_in_memory().unwrap();
        let unlocked = vault::create(&mut db, "correct horse battery", KdfParams::TEST).unwrap();
        let prefix = file_prefix(Path::new(r"C:\x\vault.db"), "8ab234f8-aa23");
        assert_eq!(prefix, "auto-vault-8ab234f8-");

        // a manual backup and another vault's copy in the same folder must survive
        fs::write(dir.path().join("nexus-vault-2026-01-01.nexus"), b"manual").unwrap();
        fs::write(
            dir.path()
                .join("auto-other-12345678-2020-01-01_00-00-00.nexus"),
            b"x",
        )
        .unwrap();

        assert_eq!(last_backup_at(&db), None);
        let base = 1_793_558_543_000;
        for i in 0..5 {
            run(
                dir.path(),
                &prefix,
                3,
                &mut db,
                &unlocked.key,
                base + i * 1000,
                true,
            )
            .unwrap();
        }
        let kept = list(dir.path(), &prefix);
        assert_eq!(kept.len(), 3);
        assert!(kept[0].to_string_lossy().ends_with("_18-42-25.nexus"));
        assert_eq!(last_backup_at(&db), Some(base + 4000));
        assert!(dir.path().join("nexus-vault-2026-01-01.nexus").exists());
        assert!(
            dir.path()
                .join("auto-other-12345678-2020-01-01_00-00-00.nexus")
                .exists()
        );
    }

    #[test]
    fn unchanged_vault_is_not_copied_again() {
        use crate::core::model::{ItemPayload, SecureNotePayload};

        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::open_in_memory().unwrap();
        let mut unlocked =
            vault::create(&mut db, "correct horse battery", KdfParams::TEST).unwrap();
        let prefix = "auto-vault-test-";
        let base = 1_793_558_543_000;
        let copy = |db: &mut Database, unlocked: &vault::UnlockedVault, at: i64| {
            run(dir.path(), prefix, 10, db, &unlocked.key, base + at, false).unwrap()
        };

        assert!(copy(&mut db, &unlocked, 0).is_some(), "first copy");
        assert!(copy(&mut db, &unlocked, 1000).is_none(), "nothing changed");
        assert_eq!(
            last_backup_at(&db),
            Some(base),
            "the copy is still the first one"
        );
        assert_eq!(
            last_checked_at(&db),
            Some(base + 1000),
            "the schedule moves on"
        );

        let id = unlocked
            .create_item(
                &mut db,
                ItemPayload::SecureNote(SecureNotePayload {
                    title: "новая".into(),
                    ..Default::default()
                }),
                None,
                false,
            )
            .unwrap();
        assert!(copy(&mut db, &unlocked, 2000).is_some(), "new item");
        unlocked.set_favorite(&mut db, &id, true).unwrap();
        assert!(copy(&mut db, &unlocked, 3000).is_some(), "flag changed");
        assert_eq!(list(dir.path(), prefix).len(), 3);

        // the copies were deleted by hand: make a new one even though nothing changed
        for file in list(dir.path(), prefix) {
            fs::remove_file(file).unwrap();
        }
        assert!(copy(&mut db, &unlocked, 4000).is_some());
    }

    #[test]
    fn due_after_the_interval_or_clock_change() {
        let hour = 3_600_000;
        assert!(is_due(None, 24, 0));
        assert!(!is_due(Some(0), 24, 23 * hour));
        assert!(is_due(Some(0), 24, 24 * hour));
        assert!(is_due(Some(10 * hour), 24, 0), "clock moved backwards");
    }
}
