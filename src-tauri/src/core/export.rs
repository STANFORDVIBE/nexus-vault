//! Plaintext export, so the user is never locked in. CSV follows the Bitwarden layout
//! (most password managers import it); JSON keeps every field.
//!
//! The output is NOT encrypted: the UI warns about it and asks for the master
//! password first. Buffers are zeroized after being written.

use serde::Serialize;
use zeroize::Zeroizing;

use crate::core::error::{AppError, AppResult};
use crate::core::model::ItemPayload;
use crate::core::vault::{UnlockedVault, VaultItem};

#[derive(Debug, Clone, Copy, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Csv,
    Json,
}

impl ExportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::Csv => "csv",
            ExportFormat::Json => "json",
        }
    }
}

/// Active (not trashed) items, sorted by title for a stable, readable file.
fn active_items(vault: &UnlockedVault) -> Vec<&VaultItem> {
    let mut items: Vec<&VaultItem> = vault
        .items
        .values()
        .filter(|i| i.deleted_at.is_none())
        .collect();
    items.sort_by_key(|i| i.payload.title().to_lowercase());
    items
}

fn folder_name<'a>(vault: &'a UnlockedVault, item: &VaultItem) -> &'a str {
    item.folder_id
        .as_deref()
        .and_then(|id| vault.folders.get(id))
        .map_or("", |f| f.name.as_str())
}

fn csv_cell(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

/// Bitwarden CSV: folder,favorite,type,name,notes,fields,reprompt,login_uri,
/// login_username,login_password,login_totp. Types it cannot hold go in as notes.
pub fn to_csv(vault: &UnlockedVault) -> Zeroizing<String> {
    let mut out = Zeroizing::new(String::from(
        "folder,favorite,type,name,notes,fields,reprompt,login_uri,login_username,login_password,login_totp\r\n",
    ));
    for item in active_items(vault) {
        let mut row: Vec<String> = vec![
            folder_name(vault, item).into(),
            if item.favorite { "1" } else { "" }.into(),
        ];
        match &item.payload {
            ItemPayload::Login(p) => {
                let fields = p
                    .email
                    .as_deref()
                    .map(|e| format!("Email: {e}"))
                    .unwrap_or_default();
                row.extend([
                    "login".into(),
                    p.title.clone(),
                    p.notes.clone().unwrap_or_default(),
                    fields,
                    String::new(),
                    p.website.clone().unwrap_or_default(),
                    p.username.clone().unwrap_or_default(),
                    p.password.clone().unwrap_or_default(),
                    String::new(),
                ]);
            }
            ItemPayload::SecureNote(p) => {
                row.extend([
                    "note".into(),
                    p.title.clone(),
                    p.content.clone(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                ]);
            }
        }
        let line: Vec<String> = row.iter().map(|c| csv_cell(c)).collect();
        out.push_str(&line.join(","));
        out.push_str("\r\n");
        for mut cell in row {
            zeroize::Zeroize::zeroize(&mut cell);
        }
    }
    out
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonExport<'a> {
    format: &'static str,
    app_version: &'static str,
    exported_at: i64,
    folders: Vec<&'a str>,
    items: Vec<JsonItem<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonItem<'a> {
    folder: Option<&'a str>,
    favorite: bool,
    created_at: i64,
    updated_at: i64,
    #[serde(flatten)]
    content: &'a ItemPayload,
}

pub fn to_json(vault: &UnlockedVault, now: i64) -> AppResult<Zeroizing<String>> {
    let mut folders: Vec<&str> = vault.folders.values().map(|f| f.name.as_str()).collect();
    folders.sort_unstable();
    let export = JsonExport {
        format: "nexus-vault-export/v1",
        app_version: env!("CARGO_PKG_VERSION"),
        exported_at: now,
        folders,
        items: active_items(vault)
            .into_iter()
            .map(|item| {
                let folder = folder_name(vault, item);
                JsonItem {
                    folder: (!folder.is_empty()).then_some(folder),
                    favorite: item.favorite,
                    created_at: item.created_at,
                    updated_at: item.updated_at,
                    content: &item.payload,
                }
            })
            .collect(),
    };
    serde_json::to_string_pretty(&export)
        .map(Zeroizing::new)
        .map_err(|_| AppError::Internal)
}

pub fn render(
    vault: &UnlockedVault,
    format: ExportFormat,
    now: i64,
) -> AppResult<Zeroizing<String>> {
    match format {
        ExportFormat::Csv => Ok(to_csv(vault)),
        ExportFormat::Json => to_json(vault, now),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::model::{LoginPayload, SecureNotePayload};
    use crate::core::vault;
    use crate::crypto::kdf::KdfParams;
    use crate::database::Database;

    fn vault_with_items() -> UnlockedVault {
        let mut db = Database::open_in_memory().unwrap();
        let mut v = vault::create(&mut db, "correct horse battery", KdfParams::TEST).unwrap();
        let folder = v.create_folder(&mut db, "Работа").unwrap();
        v.create_item(
            &mut db,
            ItemPayload::Login(LoginPayload {
                title: "GitHub".into(),
                website: Some("https://github.com".into()),
                username: Some("octo".into()),
                email: Some("me@example.com".into()),
                password: Some("p,a\"ss".into()),
                ..Default::default()
            }),
            Some(folder),
            true,
        )
        .unwrap();
        v.create_item(
            &mut db,
            ItemPayload::SecureNote(SecureNotePayload {
                title: "Wi-Fi".into(),
                content: "line1\nline2".into(),
                tags: vec![],
            }),
            None,
            false,
        )
        .unwrap();
        let trashed = v
            .create_item(
                &mut db,
                ItemPayload::SecureNote(SecureNotePayload {
                    title: "Old".into(),
                    ..Default::default()
                }),
                None,
                false,
            )
            .unwrap();
        v.move_to_trash(&mut db, &trashed).unwrap();
        v
    }

    #[test]
    fn csv_uses_bitwarden_columns_and_escapes() {
        let csv = to_csv(&vault_with_items());
        let lines: Vec<&str> = csv.split("\r\n").filter(|l| !l.is_empty()).collect();
        assert!(lines[0].starts_with("folder,favorite,type,name,notes,fields"));
        assert_eq!(lines.len(), 3, "header + 2 active items, trash skipped");
        assert_eq!(
            lines[1],
            "Работа,1,login,GitHub,,Email: me@example.com,,https://github.com,octo,\"p,a\"\"ss\","
        );
        assert!(csv.contains(",note,Wi-Fi,\"line1\nline2\","));
    }

    #[test]
    fn json_keeps_every_field() {
        let json = to_json(&vault_with_items(), 1).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["items"].as_array().unwrap().len(), 2);
        assert_eq!(value["items"][0]["type"], "login");
        assert_eq!(value["items"][0]["payload"]["password"], "p,a\"ss");
        assert_eq!(value["items"][0]["folder"], "Работа");
    }
}
