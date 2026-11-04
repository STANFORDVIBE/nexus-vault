//! Import from other password managers, so moving in is as easy as moving out.
//!
//! CSV columns are recognised by name, which covers the usual exports: Bitwarden (and
//! our own CSV), Chrome / Edge / Opera, Firefox, KeePass / KeePassXC and 1Password.
//! Our own JSON export is read back with every field and item type.
//!
//! Imported entries are always added as new items: nothing in the vault is overwritten,
//! and entries that already exist (same content) are skipped, so importing the same
//! file twice does no harm.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, Zeroizing};

use crate::core::error::{AppError, AppResult};
use crate::core::model::{CustomField, ItemPayload, LoginPayload, SecureNotePayload};
use crate::core::time::now_millis;
use crate::core::vault::{UnlockedVault, VaultItem};
use crate::crypto::totp::Totp;
use crate::crypto::{CRYPTO_VERSION, random};
use crate::database::Database;
use crate::database::repo::{self, ItemRow};

/// Bigger files are certainly not password exports.
pub const MAX_IMPORT_BYTES: u64 = 50 * 1024 * 1024;
const MAX_IMPORT_ITEMS: usize = 50_000;
const UNTITLED: &str = "Без названия";

/// One entry read from the file, not yet validated.
pub struct Entry {
    payload: ItemPayload,
    folder: Option<String>,
    favorite: bool,
}

/// What the file turned out to be and what could be read from it.
pub struct Parsed {
    pub format: &'static str,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    /// Name of the recognised source, for the result message.
    pub format: &'static str,
    pub imported: usize,
    /// Already in the vault (or repeated in the file).
    pub duplicates: usize,
    /// Rows that could not be imported (e.g. a field over the length limit).
    pub skipped: usize,
}

// ---- CSV ----------------------------------------------------------------------------

/// RFC 4180 records: quoted cells may hold commas, quotes (`""`) and line breaks.
fn csv_records(text: &str) -> Vec<Vec<String>> {
    let mut records = Vec::new();
    let mut record = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match (quoted, c) {
            (true, '"') if chars.peek() == Some(&'"') => {
                chars.next();
                cell.push('"');
            }
            (true, '"') => quoted = false,
            (true, c) => cell.push(c),
            (false, '"') => quoted = true,
            (false, ',') => record.push(std::mem::take(&mut cell)),
            (false, '\r') => {}
            (false, '\n') => {
                record.push(std::mem::take(&mut cell));
                records.push(std::mem::take(&mut record));
            }
            (false, c) => cell.push(c),
        }
    }
    if !cell.is_empty() || !record.is_empty() {
        record.push(cell);
        records.push(record);
    }
    // blank lines
    records.retain(|r| r.iter().any(|c| !c.trim().is_empty()));
    records
}

/// Column positions, found by any of the names different managers use.
struct Columns {
    title: Option<usize>,
    url: Option<usize>,
    username: Option<usize>,
    email: Option<usize>,
    password: Option<usize>,
    notes: Option<usize>,
    totp: Option<usize>,
    folder: Option<usize>,
    favorite: Option<usize>,
    kind: Option<usize>,
    fields: Option<usize>,
}

impl Columns {
    fn find(header: &[String]) -> Columns {
        let names: Vec<String> = header.iter().map(|h| h.trim().to_lowercase()).collect();
        let col = |aliases: &[&str]| {
            aliases
                .iter()
                .find_map(|a| names.iter().position(|n| n == a))
        };
        Columns {
            title: col(&["name", "title", "account"]),
            url: col(&[
                "login_uri",
                "url",
                "website",
                "web site",
                "uri",
                "login url",
            ]),
            username: col(&["login_username", "username", "user name", "login", "user"]),
            email: col(&["email", "e-mail"]),
            password: col(&["login_password", "password"]),
            notes: col(&["notes", "note", "comments", "comment", "extra"]),
            totp: col(&["login_totp", "totp", "otpauth", "otp", "one-time password"]),
            folder: col(&["folder", "group", "grouping"]),
            favorite: col(&["favorite", "favourite", "fav"]),
            kind: col(&["type"]),
            fields: col(&["fields"]),
        }
    }
}

fn detect_format(header: &[String]) -> &'static str {
    let names: HashSet<String> = header.iter().map(|h| h.trim().to_lowercase()).collect();
    let has = |n: &str| names.contains(n);
    if has("login_password") {
        "Bitwarden"
    } else if has("httprealm") {
        "Firefox"
    } else if has("group") && has("title") {
        "KeePass"
    } else if has("otpauth") {
        "1Password"
    } else if has("name") && has("url") && has("username") && has("password") {
        "Chrome / Edge"
    } else {
        "CSV"
    }
}

fn host_of(url: &str) -> Option<String> {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let host = rest
        .split(['/', '?', '#', ':'])
        .next()?
        .trim_start_matches("www.");
    (!host.is_empty()).then(|| host.to_string())
}

fn parse_csv(text: &str) -> AppResult<Parsed> {
    let mut records = csv_records(text).into_iter();
    let header = records
        .next()
        .ok_or(AppError::Validation("import_format"))?;
    let columns = Columns::find(&header);
    if columns.password.is_none() && columns.url.is_none() && columns.title.is_none() {
        return Err(AppError::Validation("import_format"));
    }
    let format = detect_format(&header);

    let mut entries = Vec::new();
    for mut record in records {
        let get = |index: Option<usize>| -> Option<String> {
            let value = record.get(index?)?.trim();
            (!value.is_empty()).then(|| value.to_string())
        };
        let title = get(columns.title);
        let url = get(columns.url);
        let username = get(columns.username);
        let mut email = get(columns.email);
        let password = get(columns.password);
        let mut notes = get(columns.notes);
        let totp = get(columns.totp);
        let favorite = get(columns.favorite)
            .is_some_and(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes"));
        // KeePass puts everything under "Root"; nested groups become "Root/Work"
        let folder = get(columns.folder).and_then(|f| {
            let f = f.strip_prefix("Root/").unwrap_or(&f).trim().to_string();
            (!f.is_empty() && f != "Root").then_some(f)
        });

        // Bitwarden "fields": one `Label: value` per line
        let mut fields: Vec<CustomField> = get(columns.fields)
            .map(|text| {
                text.lines()
                    .filter_map(|line| {
                        let (label, value) = line.split_once(':')?;
                        Some(CustomField {
                            label: label.trim().to_string(),
                            value: value.trim().to_string(),
                            hidden: false,
                        })
                    })
                    .filter(|f| !f.label.is_empty())
                    .collect()
            })
            .unwrap_or_default();
        if email.is_none()
            && let Some(at) = fields
                .iter()
                .position(|f| f.label.eq_ignore_ascii_case("email"))
        {
            email = Some(fields.remove(at).value);
        }

        let title = title
            .or_else(|| url.as_deref().and_then(host_of))
            .or_else(|| username.clone())
            .unwrap_or_else(|| UNTITLED.to_string());

        let payload = if get(columns.kind).as_deref() == Some("note") {
            let mut content = notes.take().unwrap_or_default();
            for f in &fields {
                content.push_str(&format!("\n{}: {}", f.label, f.value));
            }
            ItemPayload::SecureNote(SecureNotePayload {
                title,
                content: content.trim().to_string(),
                tags: Vec::new(),
            })
        } else {
            // a code we cannot generate (e.g. Steam) is kept as a hidden field
            let totp = match totp {
                Some(secret) if Totp::parse(&secret).is_err() => {
                    fields.push(CustomField {
                        label: "2FA".into(),
                        value: secret,
                        hidden: true,
                    });
                    None
                }
                other => other,
            };
            ItemPayload::Login(LoginPayload {
                title,
                website: url,
                username,
                email,
                password,
                totp,
                notes,
                fields,
                ..Default::default()
            })
        };
        entries.push(Entry {
            payload,
            folder,
            favorite,
        });
        for cell in &mut record {
            cell.zeroize();
        }
    }
    Ok(Parsed { format, entries })
}

// ---- our own JSON export ---------------------------------------------------------------

#[derive(Deserialize)]
struct JsonImport {
    format: String,
    items: Vec<JsonImportItem>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsonImportItem {
    #[serde(default)]
    folder: Option<String>,
    #[serde(default)]
    favorite: bool,
    #[serde(flatten)]
    content: ItemPayload,
}

fn parse_json(text: &str) -> AppResult<Parsed> {
    let export: JsonImport =
        serde_json::from_str(text).map_err(|_| AppError::Validation("import_format"))?;
    if export.format != "nexus-vault-export/v1" {
        return Err(AppError::Validation("import_format"));
    }
    let entries = export
        .items
        .into_iter()
        .map(|item| Entry {
            payload: item.content,
            folder: item.folder,
            favorite: item.favorite,
        })
        .collect();
    Ok(Parsed {
        format: "Nexus Vault",
        entries,
    })
}

/// Reads an export file. Fails only when the file is not a recognisable export.
pub fn parse(bytes: &[u8]) -> AppResult<Parsed> {
    let text = std::str::from_utf8(bytes).map_err(|_| AppError::Validation("import_format"))?;
    let text = text.trim_start_matches('\u{feff}');
    let parsed = if text.trim_start().starts_with('{') {
        parse_json(text)?
    } else {
        parse_csv(text)?
    };
    if parsed.entries.len() > MAX_IMPORT_ITEMS {
        return Err(AppError::Validation("import_too_large"));
    }
    Ok(parsed)
}

// ---- adding to the vault -----------------------------------------------------------------

/// Content fingerprint for duplicate detection: a hash of the item without its
/// bookkeeping (a hash, so no extra plaintext copies stay around).
fn dedupe_key(payload: &ItemPayload) -> [u8; 32] {
    let mut payload = payload.clone();
    if let ItemPayload::Login(p) = &mut payload {
        p.password_changed_at = None;
        p.password_history.clear();
    }
    let json = Zeroizing::new(serde_json::to_vec(&payload).unwrap_or_default());
    Sha256::digest(&*json).into()
}

impl UnlockedVault {
    /// Adds parsed entries as new items in one transaction; missing folders are created.
    pub fn import(&mut self, db: &mut Database, parsed: Parsed) -> AppResult<ImportSummary> {
        let mut summary = ImportSummary {
            format: parsed.format,
            imported: 0,
            duplicates: 0,
            skipped: 0,
        };
        let mut seen: HashSet<[u8; 32]> = self
            .items
            .values()
            .filter(|i| i.deleted_at.is_none())
            .map(|i| dedupe_key(&i.payload))
            .collect();
        let mut folder_ids: HashMap<String, String> = self
            .folders
            .values()
            .map(|f| (f.name.to_lowercase(), f.id.clone()))
            .collect();

        let now = now_millis();
        let mut rows = Vec::new();
        let mut items = Vec::new();
        for mut entry in parsed.entries {
            if entry.payload.validate_and_normalize().is_err() {
                summary.skipped += 1;
                continue;
            }
            if !seen.insert(dedupe_key(&entry.payload)) {
                summary.duplicates += 1;
                continue;
            }
            let folder_id = match entry.folder.as_deref().map(str::trim) {
                Some(name) if !name.is_empty() => match folder_ids.get(&name.to_lowercase()) {
                    Some(id) => Some(id.clone()),
                    // an invalid folder name (too long) just leaves the item unfiled
                    None => self.create_folder(db, name).ok().inspect(|id| {
                        folder_ids.insert(name.to_lowercase(), id.clone());
                    }),
                },
                _ => None,
            };
            if let ItemPayload::Login(p) = &mut entry.payload {
                p.password_changed_at = p.password.is_some().then_some(now);
                p.password_history.clear();
            }

            let id = random::new_id()?;
            let sealed = self.seal_payload(&id, &entry.payload)?;
            rows.push(ItemRow {
                id: id.clone(),
                item_type: entry.payload.item_type().as_str().into(),
                encrypted_payload: sealed.ciphertext,
                nonce: sealed.nonce.to_vec(),
                crypto_version: CRYPTO_VERSION,
                folder_id: folder_id.clone(),
                is_favorite: entry.favorite,
                created_at: now,
                updated_at: now,
                deleted_at: None,
            });
            items.push(VaultItem {
                id,
                item_type: entry.payload.item_type(),
                folder_id,
                favorite: entry.favorite,
                created_at: now,
                updated_at: now,
                deleted_at: None,
                payload: entry.payload.clone(),
            });
        }

        db.transaction(|tx| rows.iter().try_for_each(|row| repo::insert_item(tx, row)))?;
        summary.imported = items.len();
        for item in items {
            self.items.insert(item.id.clone(), item);
        }
        Ok(summary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::export;
    use crate::core::vault;
    use crate::crypto::kdf::KdfParams;

    fn setup() -> (Database, UnlockedVault) {
        let mut db = Database::open_in_memory().unwrap();
        let vault = vault::create(&mut db, "correct horse battery", KdfParams::TEST).unwrap();
        (db, vault)
    }

    fn logins(vault: &UnlockedVault) -> Vec<&LoginPayload> {
        let mut out: Vec<&LoginPayload> = vault
            .items
            .values()
            .filter_map(|i| match &i.payload {
                ItemPayload::Login(p) => Some(p),
                _ => None,
            })
            .collect();
        out.sort_by(|a, b| a.title.cmp(&b.title));
        out
    }

    #[test]
    fn reads_quoted_csv_cells() {
        let records = csv_records("a,b\r\n\"x, y\",\"say \"\"hi\"\"\nnext line\"\n\n1,2");
        assert_eq!(records.len(), 3);
        assert_eq!(records[1], vec!["x, y", "say \"hi\"\nnext line"]);
        assert_eq!(records[2], vec!["1", "2"]);
    }

    #[test]
    fn imports_chrome_csv() {
        let (mut db, mut vault) = setup();
        let csv = "\u{feff}name,url,username,password,note\n\
                   github.com,https://github.com/login,me,s3cret,\n\
                   ,https://www.example.org/,bob,pw,remember me\n";
        let parsed = parse(csv.as_bytes()).unwrap();
        assert_eq!(parsed.format, "Chrome / Edge");
        let summary = vault.import(&mut db, parsed).unwrap();
        assert_eq!(summary.imported, 2);
        let items = logins(&vault);
        assert_eq!(
            items[0].title, "example.org",
            "untitled rows are named after the site"
        );
        assert_eq!(items[0].notes.as_deref(), Some("remember me"));
        assert_eq!(items[1].password.as_deref(), Some("s3cret"));
        assert!(items[1].password_changed_at.is_some());
    }

    #[test]
    fn imports_bitwarden_csv_with_folders_notes_fields_and_totp() {
        let (mut db, mut vault) = setup();
        let csv = "folder,favorite,type,name,notes,fields,reprompt,login_uri,login_username,login_password,login_totp\n\
                   Work,1,login,Mail,,\"Email: me@x.ru\nPIN: 1234\",0,https://mail.ru,me,pw,GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ\n\
                   work,,login,Steam,,,0,,me,pw,steam://ABCDE\n\
                   ,,note,Wi-Fi,password is 123,,,,,,\n";
        let summary = vault
            .import(&mut db, parse(csv.as_bytes()).unwrap())
            .unwrap();
        assert_eq!(summary.format, "Bitwarden");
        assert_eq!(summary.imported, 3);
        assert_eq!(
            vault.folders.len(),
            1,
            "folder names match case-insensitively"
        );
        let items = logins(&vault);
        let mail = items[0];
        assert_eq!(mail.email.as_deref(), Some("me@x.ru"));
        assert_eq!(mail.fields.len(), 1);
        assert_eq!(mail.fields[0].label, "PIN");
        assert!(mail.totp.is_some());
        let steam = items[1];
        assert!(steam.totp.is_none());
        assert!(steam.fields.iter().any(|f| f.label == "2FA" && f.hidden));
        assert!(vault.items.values().any(|i| i.favorite));
        assert!(vault.items.values().any(|i| matches!(
            &i.payload,
            ItemPayload::SecureNote(n) if n.content == "password is 123"
        )));
    }

    #[test]
    fn importing_twice_adds_nothing_new() {
        let (mut db, mut vault) = setup();
        let csv = "url,username,password,httpRealm\nhttps://a.com,me,pw,\nhttps://a.com,me,pw,\n";
        let first = vault
            .import(&mut db, parse(csv.as_bytes()).unwrap())
            .unwrap();
        assert_eq!(first.format, "Firefox");
        assert_eq!((first.imported, first.duplicates), (1, 1));
        let second = vault
            .import(&mut db, parse(csv.as_bytes()).unwrap())
            .unwrap();
        assert_eq!((second.imported, second.duplicates), (0, 2));
        // and the items really are in the database
        let reopened = vault::unlock(&db, "correct horse battery").unwrap();
        assert_eq!(reopened.items.len(), 1);
    }

    #[test]
    fn bad_rows_are_skipped_not_fatal() {
        let (mut db, mut vault) = setup();
        let long = "x".repeat(5000);
        let csv = format!(
            "Title,Username,Password,URL,Notes,Group\nok,me,pw,,,Root\nbig,me,{long},,,Root/Work\n"
        );
        let summary = vault
            .import(&mut db, parse(csv.as_bytes()).unwrap())
            .unwrap();
        assert_eq!(summary.format, "KeePass");
        assert_eq!((summary.imported, summary.skipped), (1, 1));
        assert!(vault.folders.is_empty(), "KeePass Root is not a folder");
    }

    #[test]
    fn rejects_files_that_are_not_exports() {
        assert!(parse(b"").is_err());
        assert!(parse(b"just,some,columns\n1,2,3").is_err());
        assert!(parse(b"{\"format\":\"other\",\"items\":[]}").is_err());
        assert!(parse(&[0xff, 0xfe, 0x00]).is_err());
    }

    #[test]
    fn own_json_export_round_trips_every_type() {
        let (mut db, mut vault) = setup();
        let csv = "folder,favorite,type,name,notes,fields,reprompt,login_uri,login_username,login_password,login_totp\n\
                   Work,1,login,Mail,,,0,https://mail.ru,me,pw,GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ\n\
                   ,,note,Wi-Fi,password is 123,,,,,,\n";
        vault
            .import(&mut db, parse(csv.as_bytes()).unwrap())
            .unwrap();
        let json = export::to_json(&vault, 0).unwrap();

        let (mut db2, mut other) = setup();
        let summary = other
            .import(&mut db2, parse(json.as_bytes()).unwrap())
            .unwrap();
        assert_eq!(summary.format, "Nexus Vault");
        assert_eq!(summary.imported, 2);
        assert_eq!(other.folders.len(), 1);
        assert_eq!(logins(&other)[0].totp, logins(&vault)[0].totp);
        // and back into the original vault: everything is already there
        let again = vault
            .import(&mut db, parse(json.as_bytes()).unwrap())
            .unwrap();
        assert_eq!((again.imported, again.duplicates), (0, 2));
    }
}
