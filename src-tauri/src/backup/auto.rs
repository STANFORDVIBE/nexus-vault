//! Automatic backups: an encrypted `.nexus` copy every N hours while the vault is
//! unlocked, keeping the newest `keep` files. Only files this module wrote
//! (`auto-<vault>-<id>-<timestamp>.nexus`) are ever rotated away.

use std::fs;
use std::path::{Path, PathBuf};

use crate::backup::{EXTENSION, write_backup};
use crate::core::error::{AppError, AppResult};
use crate::core::time::civil_utc;
use crate::crypto::aead::SecretKey;
use crate::database::Database;
use crate::database::repo;

/// Plaintext `settings` key with the time of the last automatic copy (per vault).
const LAST_KEY: &str = "last_auto_backup_at";

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

pub fn last_backup_at(db: &Database) -> Option<i64> {
    repo::load_settings(db.conn())
        .ok()?
        .into_iter()
        .find(|(k, _)| k == LAST_KEY)
        .and_then(|(_, v)| v.parse().ok())
}

/// Writes one automatic copy, records its time and rotates old ones.
pub fn run(
    folder: &Path,
    prefix: &str,
    keep: usize,
    db: &mut Database,
    key: &SecretKey,
    now: i64,
) -> AppResult<PathBuf> {
    fs::create_dir_all(folder)?;
    let path = folder.join(file_name(prefix, now));
    if path.exists() {
        // two runs within one second: the existing copy is just as fresh
        return Ok(path);
    }
    write_backup(&path, db, key)?;
    db.transaction(|tx| repo::set_setting(tx, LAST_KEY, &now.to_string()))?;
    rotate(folder, prefix, keep.max(1));
    Ok(path)
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
    fn due_after_the_interval_or_clock_change() {
        let hour = 3_600_000;
        assert!(is_due(None, 24, 0));
        assert!(!is_due(Some(0), 24, 23 * hour));
        assert!(is_due(Some(0), 24, 24 * hour));
        assert!(is_due(Some(10 * hour), 24, 0), "clock moved backwards");
    }
}
