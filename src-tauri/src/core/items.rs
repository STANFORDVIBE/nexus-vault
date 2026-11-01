//! Item operations on an unlocked vault: CRUD, favorites, trash, filtering and search.
//! Every write goes to SQLite first (inside a transaction) and only then to the
//! in-memory cache, so a failed write never leaves the two out of sync.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::core::error::{AppError, AppResult};
use crate::core::model::{ItemPayload, ItemType};
use crate::core::time::now_millis;
use crate::core::vault::{UnlockedVault, VaultItem, encode_payload, item_aad};
use crate::crypto::{CRYPTO_VERSION, aead, random};
use crate::database::Database;
use crate::database::repo::{self, ItemRow};

// ---- views sent to the frontend ---------------------------------------------------

/// List row. Contains no secret fields (no password, note body or login notes).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemSummary {
    pub id: String,
    #[serde(rename = "type")]
    pub item_type: ItemType,
    pub title: String,
    pub subtitle: Option<String>,
    pub website: Option<String>,
    pub favorite: bool,
    pub folder_id: Option<String>,
    pub tags: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
}

/// Detail view. The login password is never included: the UI asks for it explicitly
/// (`reveal_password`) or lets Rust copy it straight to the clipboard.
#[derive(Serialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum ItemContentView {
    #[serde(rename_all = "camelCase")]
    Login {
        title: String,
        website: Option<String>,
        username: Option<String>,
        email: Option<String>,
        notes: Option<String>,
        tags: Vec<String>,
        has_password: bool,
    },
    #[serde(rename_all = "camelCase")]
    SecureNote {
        title: String,
        content: String,
        tags: Vec<String>,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemDetail {
    pub id: String,
    pub favorite: bool,
    pub folder_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
    #[serde(flatten)]
    pub content: ItemContentView,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ItemView {
    All,
    Logins,
    Notes,
    Favorites,
    Trash,
    Folder,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ItemSort {
    #[default]
    Updated,
    Created,
    Title,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemFilter {
    pub view: ItemView,
    #[serde(default)]
    pub folder_id: Option<String>,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub sort: ItemSort,
}

/// Fields that can be copied to the clipboard by the backend.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CopyField {
    Username,
    Email,
    Password,
    Website,
    Content,
}

impl CopyField {
    pub fn is_secret(self) -> bool {
        matches!(self, CopyField::Password | CopyField::Content)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardData {
    pub total: usize,
    pub logins: usize,
    pub notes: usize,
    pub favorites: usize,
    pub trash: usize,
    pub recent: Vec<ItemSummary>,
    pub favorite_items: Vec<ItemSummary>,
}

// ---- helpers --------------------------------------------------------------------------

fn host_of(url: &str) -> String {
    let without_scheme = url.split_once("://").map_or(url, |(_, rest)| rest);
    let host = without_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(without_scheme);
    host.trim_start_matches("www.").to_string()
}

fn first_line_preview(markdown: &str) -> Option<String> {
    let line = markdown
        .lines()
        .map(|l| {
            l.trim()
                .trim_start_matches(['#', '>', '-', '*', ' '])
                .trim()
        })
        .find(|l| !l.is_empty())?;
    let preview: String = line.chars().take(80).collect();
    Some(preview)
}

fn summary(item: &VaultItem) -> ItemSummary {
    let (subtitle, website) = match &item.payload {
        ItemPayload::Login(p) => (
            p.username
                .clone()
                .or_else(|| p.email.clone())
                .or_else(|| p.website.as_deref().map(host_of)),
            p.website.clone(),
        ),
        ItemPayload::SecureNote(p) => (first_line_preview(&p.content), None),
    };
    ItemSummary {
        id: item.id.clone(),
        item_type: item.item_type,
        title: item.payload.title().to_string(),
        subtitle,
        website,
        favorite: item.favorite,
        folder_id: item.folder_id.clone(),
        tags: item.payload.tags().to_vec(),
        created_at: item.created_at,
        updated_at: item.updated_at,
        deleted_at: item.deleted_at,
    }
}

fn detail(item: &VaultItem) -> ItemDetail {
    let content = match &item.payload {
        ItemPayload::Login(p) => ItemContentView::Login {
            title: p.title.clone(),
            website: p.website.clone(),
            username: p.username.clone(),
            email: p.email.clone(),
            notes: p.notes.clone(),
            tags: p.tags.clone(),
            has_password: p.password.is_some(),
        },
        ItemPayload::SecureNote(p) => ItemContentView::SecureNote {
            title: p.title.clone(),
            content: p.content.clone(),
            tags: p.tags.clone(),
        },
    };
    ItemDetail {
        id: item.id.clone(),
        favorite: item.favorite,
        folder_id: item.folder_id.clone(),
        created_at: item.created_at,
        updated_at: item.updated_at,
        deleted_at: item.deleted_at,
        content,
    }
}

/// Lower-cased searchable text of an item (title first so it can be ranked).
fn haystack(item: &VaultItem) -> (String, String) {
    let title = item.payload.title().to_lowercase();
    let mut rest = item.payload.tags().join(" ");
    match &item.payload {
        ItemPayload::Login(p) => {
            for field in [&p.website, &p.username, &p.email, &p.notes]
                .into_iter()
                .flatten()
            {
                rest.push(' ');
                rest.push_str(field);
            }
        }
        ItemPayload::SecureNote(p) => {
            rest.push(' ');
            rest.push_str(&p.content);
        }
    }
    (title, rest.to_lowercase())
}

/// 0 = no match, higher = better. Every query term must appear somewhere.
fn match_score(item: &VaultItem, terms: &[String]) -> u8 {
    let (title, rest) = haystack(item);
    let mut score = 1;
    for term in terms {
        if title.contains(term.as_str()) {
            score = score.max(if title.starts_with(term.as_str()) {
                3
            } else {
                2
            });
        } else if !rest.contains(term.as_str()) {
            return 0;
        }
    }
    score
}

impl UnlockedVault {
    fn item(&self, id: &str) -> AppResult<&VaultItem> {
        self.items.get(id).ok_or(AppError::NotFound)
    }

    fn ensure_folder(&self, folder_id: Option<&str>) -> AppResult<()> {
        match folder_id {
            Some(id) if !self.folders.contains_key(id) => Err(AppError::Validation("folder")),
            _ => Ok(()),
        }
    }

    fn seal_payload(&self, id: &str, payload: &ItemPayload) -> AppResult<aead::Sealed> {
        let plain = encode_payload(payload)?;
        aead::seal(
            &self.key,
            &plain,
            &item_aad(&self.vault_id, id, payload.item_type()),
        )
    }

    pub fn list_items(&self, filter: &ItemFilter) -> Vec<ItemSummary> {
        let terms: Vec<String> = filter
            .query
            .as_deref()
            .unwrap_or("")
            .split_whitespace()
            .map(str::to_lowercase)
            .collect();

        let mut matches: Vec<(u8, &VaultItem)> = self
            .items
            .values()
            .filter(|item| match filter.view {
                ItemView::Trash => item.deleted_at.is_some(),
                _ if item.deleted_at.is_some() => false,
                ItemView::All => true,
                ItemView::Logins => item.item_type == ItemType::Login,
                ItemView::Notes => item.item_type == ItemType::SecureNote,
                ItemView::Favorites => item.favorite,
                ItemView::Folder => item.folder_id.as_deref() == filter.folder_id.as_deref(),
            })
            .filter_map(|item| {
                if terms.is_empty() {
                    Some((0, item))
                } else {
                    let score = match_score(item, &terms);
                    (score > 0).then_some((score, item))
                }
            })
            .collect();

        let titles = |a: &VaultItem, b: &VaultItem| {
            a.payload
                .title()
                .to_lowercase()
                .cmp(&b.payload.title().to_lowercase())
        };
        matches.sort_by(|(sa, a), (sb, b)| {
            sb.cmp(sa).then_with(|| match filter.sort {
                ItemSort::Updated => b.updated_at.cmp(&a.updated_at).then_with(|| titles(a, b)),
                ItemSort::Created => b.created_at.cmp(&a.created_at).then_with(|| titles(a, b)),
                ItemSort::Title => titles(a, b).then(Ordering::Equal),
            })
        });
        matches.into_iter().map(|(_, item)| summary(item)).collect()
    }

    pub fn get_item(&self, id: &str) -> AppResult<ItemDetail> {
        Ok(detail(self.item(id)?))
    }

    pub fn dashboard(&self) -> DashboardData {
        let active: Vec<&VaultItem> = self
            .items
            .values()
            .filter(|i| i.deleted_at.is_none())
            .collect();
        let mut recent: Vec<&VaultItem> = active.clone();
        recent.sort_by_key(|i| std::cmp::Reverse(i.updated_at));
        let mut favorites: Vec<&VaultItem> =
            active.iter().copied().filter(|i| i.favorite).collect();
        favorites.sort_by_key(|i| std::cmp::Reverse(i.updated_at));

        DashboardData {
            total: active.len(),
            logins: active
                .iter()
                .filter(|i| i.item_type == ItemType::Login)
                .count(),
            notes: active
                .iter()
                .filter(|i| i.item_type == ItemType::SecureNote)
                .count(),
            favorites: favorites.len(),
            trash: self.items.len() - active.len(),
            recent: recent.into_iter().take(6).map(summary).collect(),
            favorite_items: favorites.into_iter().take(6).map(summary).collect(),
        }
    }

    /// Plaintext of a single field, for reveal/copy. Secret values stay in Rust otherwise.
    pub fn field_value(&self, id: &str, field: CopyField) -> AppResult<String> {
        let item = self.item(id)?;
        let value = match (&item.payload, field) {
            (ItemPayload::Login(p), CopyField::Username) => p.username.clone(),
            (ItemPayload::Login(p), CopyField::Email) => p.email.clone(),
            (ItemPayload::Login(p), CopyField::Password) => p.password.clone(),
            (ItemPayload::Login(p), CopyField::Website) => p.website.clone(),
            (ItemPayload::SecureNote(p), CopyField::Content) => Some(p.content.clone()),
            _ => None,
        };
        value.ok_or(AppError::NotFound)
    }

    pub fn create_item(
        &mut self,
        db: &mut Database,
        mut payload: ItemPayload,
        folder_id: Option<String>,
        favorite: bool,
    ) -> AppResult<String> {
        payload.validate_and_normalize()?;
        self.ensure_folder(folder_id.as_deref())?;

        let id = random::new_id()?;
        let sealed = self.seal_payload(&id, &payload)?;
        let now = now_millis();
        let row = ItemRow {
            id: id.clone(),
            item_type: payload.item_type().as_str().into(),
            encrypted_payload: sealed.ciphertext,
            nonce: sealed.nonce.to_vec(),
            crypto_version: CRYPTO_VERSION,
            folder_id: folder_id.clone(),
            is_favorite: favorite,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        };
        db.transaction(|tx| repo::insert_item(tx, &row))?;

        self.items.insert(
            id.clone(),
            VaultItem {
                id: id.clone(),
                item_type: payload.item_type(),
                folder_id,
                favorite,
                created_at: now,
                updated_at: now,
                deleted_at: None,
                payload,
            },
        );
        Ok(id)
    }

    pub fn update_item(
        &mut self,
        db: &mut Database,
        id: &str,
        mut payload: ItemPayload,
        folder_id: Option<String>,
    ) -> AppResult<()> {
        let item_type = self.item(id)?.item_type;
        if payload.item_type() != item_type {
            return Err(AppError::Validation("type"));
        }
        payload.validate_and_normalize()?;
        self.ensure_folder(folder_id.as_deref())?;

        let sealed = self.seal_payload(id, &payload)?;
        let now = now_millis();
        db.transaction(|tx| {
            repo::update_item_payload(
                tx,
                id,
                &sealed.ciphertext,
                &sealed.nonce,
                CRYPTO_VERSION,
                folder_id.as_deref(),
                now,
            )?
            .then_some(())
            .ok_or(AppError::NotFound)
        })?;

        let item = self.items.get_mut(id).ok_or(AppError::NotFound)?;
        item.payload = payload;
        item.folder_id = folder_id;
        item.updated_at = now;
        Ok(())
    }

    pub fn set_favorite(&mut self, db: &mut Database, id: &str, favorite: bool) -> AppResult<()> {
        self.item(id)?;
        db.transaction(|tx| repo::set_item_favorite(tx, id, favorite))?;
        if let Some(item) = self.items.get_mut(id) {
            item.favorite = favorite;
        }
        Ok(())
    }

    pub fn move_to_folder(
        &mut self,
        db: &mut Database,
        id: &str,
        folder_id: Option<String>,
    ) -> AppResult<()> {
        self.item(id)?;
        self.ensure_folder(folder_id.as_deref())?;
        let now = now_millis();
        db.transaction(|tx| repo::set_item_folder(tx, id, folder_id.as_deref(), now))?;
        if let Some(item) = self.items.get_mut(id) {
            item.folder_id = folder_id;
            item.updated_at = now;
        }
        Ok(())
    }

    pub fn move_to_trash(&mut self, db: &mut Database, id: &str) -> AppResult<()> {
        self.item(id)?;
        let now = now_millis();
        db.transaction(|tx| repo::set_item_deleted(tx, id, Some(now)))?;
        if let Some(item) = self.items.get_mut(id) {
            item.deleted_at = Some(now);
        }
        Ok(())
    }

    pub fn restore_item(&mut self, db: &mut Database, id: &str) -> AppResult<()> {
        let folder_missing = self
            .item(id)?
            .folder_id
            .as_deref()
            .is_some_and(|f| !self.folders.contains_key(f));
        db.transaction(|tx| repo::set_item_deleted(tx, id, None))?;
        if let Some(item) = self.items.get_mut(id) {
            item.deleted_at = None;
            if folder_missing {
                item.folder_id = None;
            }
        }
        Ok(())
    }

    pub fn delete_permanently(&mut self, db: &mut Database, id: &str) -> AppResult<()> {
        if self.item(id)?.deleted_at.is_none() {
            // only trashed items can be destroyed, as a guard against UI mistakes
            return Err(AppError::Validation("not_in_trash"));
        }
        db.transaction(|tx| repo::delete_item(tx, id))?;
        self.items.remove(id);
        Ok(())
    }

    pub fn empty_trash(&mut self, db: &mut Database) -> AppResult<usize> {
        let removed = db.transaction(|tx| repo::delete_trashed_items(tx))?;
        self.items.retain(|_, item| item.deleted_at.is_none());
        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::model::{LoginPayload, SecureNotePayload};
    use crate::core::vault;
    use crate::crypto::kdf::KdfParams;

    const PASSWORD: &str = "correct horse battery";

    fn setup() -> (Database, UnlockedVault) {
        let mut db = Database::open_in_memory().unwrap();
        let vault = vault::create(&mut db, PASSWORD, KdfParams::TEST).unwrap();
        (db, vault)
    }

    fn login(title: &str, username: &str, password: &str) -> ItemPayload {
        ItemPayload::Login(LoginPayload {
            title: title.into(),
            website: Some("https://www.github.com/login".into()),
            username: Some(username.into()),
            password: Some(password.into()),
            tags: vec!["work".into()],
            ..Default::default()
        })
    }

    fn note(title: &str, content: &str) -> ItemPayload {
        ItemPayload::SecureNote(SecureNotePayload {
            title: title.into(),
            content: content.into(),
            tags: vec![],
        })
    }

    fn all() -> ItemFilter {
        ItemFilter {
            view: ItemView::All,
            folder_id: None,
            query: None,
            sort: ItemSort::Title,
        }
    }

    #[test]
    fn created_items_survive_lock_and_unlock() {
        let (mut db, mut vault) = setup();
        let id = vault
            .create_item(&mut db, login("GitHub", "octocat", "s3cr3t!"), None, true)
            .unwrap();
        vault
            .create_item(&mut db, note("Wi-Fi", "# Home\npassword"), None, false)
            .unwrap();
        drop(vault);

        let vault = vault::unlock(&db, PASSWORD).unwrap();
        assert_eq!(vault.items.len(), 2);
        assert_eq!(vault.corrupted_items, 0);
        assert_eq!(
            vault.field_value(&id, CopyField::Password).unwrap(),
            "s3cr3t!"
        );
        let detail = vault.get_item(&id).unwrap();
        assert!(detail.favorite);
    }

    #[test]
    fn secrets_are_not_stored_in_plaintext() {
        let (mut db, mut vault) = setup();
        vault
            .create_item(
                &mut db,
                login("VerySecretBank", "alice@example.com", "Pa55w0rd-XYZ"),
                None,
                false,
            )
            .unwrap();

        let rows = repo::load_items(db.conn()).unwrap();
        let blob = &rows[0].encrypted_payload;
        for needle in [
            "VerySecretBank",
            "alice@example.com",
            "Pa55w0rd-XYZ",
            "github",
        ] {
            assert!(
                !blob.windows(needle.len()).any(|w| w == needle.as_bytes()),
                "{needle} leaked"
            );
        }
    }

    #[test]
    fn summaries_and_details_never_expose_the_password() {
        let (mut db, mut vault) = setup();
        let id = vault
            .create_item(
                &mut db,
                login("GitHub", "octocat", "Pa55w0rd-XYZ"),
                None,
                false,
            )
            .unwrap();
        let list = serde_json::to_string(&vault.list_items(&all())).unwrap();
        let detail = serde_json::to_string(&vault.get_item(&id).unwrap()).unwrap();
        assert!(!list.contains("Pa55w0rd-XYZ"));
        assert!(!detail.contains("Pa55w0rd-XYZ"));
        assert!(detail.contains("\"hasPassword\":true"));
        assert!(list.contains("\"subtitle\":\"octocat\""));
    }

    #[test]
    fn swapped_ciphertexts_fail_authentication() {
        let (mut db, mut vault) = setup();
        let a = vault
            .create_item(&mut db, login("A", "a", "pa"), None, false)
            .unwrap();
        let b = vault
            .create_item(&mut db, login("B", "b", "pb"), None, false)
            .unwrap();
        // Attacker copies item A's ciphertext into item B's row
        db.conn()
            .execute(
                "UPDATE items SET encrypted_payload = (SELECT encrypted_payload FROM items WHERE id = ?1),
                     nonce = (SELECT nonce FROM items WHERE id = ?1) WHERE id = ?2",
                [&a, &b],
            )
            .unwrap();
        drop(vault);

        let vault = vault::unlock(&db, PASSWORD).unwrap();
        assert_eq!(vault.corrupted_items, 1);
        assert!(vault.items.contains_key(&a));
        assert!(!vault.items.contains_key(&b));
    }

    #[test]
    fn update_changes_content_and_rejects_type_change() {
        let (mut db, mut vault) = setup();
        let id = vault
            .create_item(&mut db, login("Old", "u", "p"), None, false)
            .unwrap();
        vault
            .update_item(&mut db, &id, login("New", "u2", "p2"), None)
            .unwrap();
        assert_eq!(vault.get_item(&id).unwrap().content_title(), "New");
        assert!(matches!(
            vault.update_item(&mut db, &id, note("x", ""), None),
            Err(AppError::Validation("type"))
        ));
        assert!(matches!(
            vault.update_item(&mut db, "missing", login("x", "", ""), None),
            Err(AppError::NotFound)
        ));
    }

    #[test]
    fn trash_restore_and_permanent_delete() {
        let (mut db, mut vault) = setup();
        let id = vault
            .create_item(&mut db, login("A", "a", "p"), None, false)
            .unwrap();

        assert!(vault.delete_permanently(&mut db, &id).is_err());
        vault.move_to_trash(&mut db, &id).unwrap();
        assert!(vault.list_items(&all()).is_empty());
        let trash = ItemFilter {
            view: ItemView::Trash,
            ..all()
        };
        assert_eq!(vault.list_items(&trash).len(), 1);

        vault.restore_item(&mut db, &id).unwrap();
        assert_eq!(vault.list_items(&all()).len(), 1);

        vault.move_to_trash(&mut db, &id).unwrap();
        assert_eq!(vault.empty_trash(&mut db).unwrap(), 1);
        assert!(vault.items.is_empty());
        assert!(repo::load_items(db.conn()).unwrap().is_empty());
    }

    #[test]
    fn search_matches_all_terms_and_ranks_titles() {
        let (mut db, mut vault) = setup();
        vault
            .create_item(&mut db, login("GitHub", "octocat", "p"), None, false)
            .unwrap();
        vault
            .create_item(&mut db, login("Work mail", "octo@corp", "p"), None, false)
            .unwrap();
        vault
            .create_item(
                &mut db,
                note("Recipes", "Grandma's octopus salad"),
                None,
                false,
            )
            .unwrap();

        let search = |q: &str| {
            vault
                .list_items(&ItemFilter {
                    query: Some(q.into()),
                    ..all()
                })
                .into_iter()
                .map(|s| s.title)
                .collect::<Vec<_>>()
        };
        assert_eq!(search("octo").len(), 3);
        // "Work mail" matches through its github.com website; the title match ranks first
        assert_eq!(search("GIT"), vec!["GitHub", "Work mail"]);
        assert_eq!(search("octo salad"), vec!["Recipes"]);
        assert!(search("nothing-matches").is_empty());
    }

    #[test]
    fn filters_by_type_favorite_and_folder() {
        let (mut db, mut vault) = setup();
        let folder = vault.create_folder(&mut db, "Работа").unwrap();
        vault
            .create_item(&mut db, login("A", "a", "p"), Some(folder.clone()), true)
            .unwrap();
        vault
            .create_item(&mut db, note("B", ""), None, false)
            .unwrap();

        let count = |view, folder_id: Option<String>| {
            vault
                .list_items(&ItemFilter {
                    view,
                    folder_id,
                    ..all()
                })
                .len()
        };
        assert_eq!(count(ItemView::Logins, None), 1);
        assert_eq!(count(ItemView::Notes, None), 1);
        assert_eq!(count(ItemView::Favorites, None), 1);
        assert_eq!(count(ItemView::Folder, Some(folder)), 1);
        assert!(
            vault
                .create_item(&mut db, note("C", ""), Some("nope".into()), false)
                .is_err()
        );
    }

    #[test]
    fn dashboard_counts() {
        let (mut db, mut vault) = setup();
        vault
            .create_item(&mut db, login("A", "a", "p"), None, true)
            .unwrap();
        vault
            .create_item(&mut db, note("B", ""), None, false)
            .unwrap();
        let c = vault
            .create_item(&mut db, note("C", ""), None, false)
            .unwrap();
        vault.move_to_trash(&mut db, &c).unwrap();

        let d = vault.dashboard();
        assert_eq!(
            (d.total, d.logins, d.notes, d.favorites, d.trash),
            (2, 1, 1, 1, 1)
        );
        assert_eq!(d.recent.len(), 2);
    }

    #[test]
    fn subtitles() {
        assert_eq!(host_of("https://www.github.com/login?x=1"), "github.com");
        assert_eq!(host_of("example.org"), "example.org");
        assert_eq!(
            first_line_preview("\n\n# Title here\nbody").as_deref(),
            Some("Title here")
        );
    }

    impl ItemDetail {
        fn content_title(&self) -> &str {
            match &self.content {
                ItemContentView::Login { title, .. }
                | ItemContentView::SecureNote { title, .. } => title,
            }
        }
    }

    /// Unlock, list and search timings for a 10 000-item vault (spec target: search < 100 ms).
    /// Run with: cargo test --release -- --ignored large_vault_benchmark --nocapture
    #[test]
    #[ignore]
    #[allow(clippy::print_stdout)]
    fn large_vault_benchmark() {
        use std::time::Instant;

        let (mut db, mut vault) = setup();
        let start = Instant::now();
        for i in 0..10_000 {
            let payload = if i % 4 == 0 {
                note(
                    &format!("Note {i}"),
                    &format!(
                        "recovery codes for service {i}
- one
- two"
                    ),
                )
            } else {
                login(
                    &format!("Site {i}"),
                    &format!("user{i}@example.com"),
                    "p@ssw0rd",
                )
            };
            vault
                .create_item(&mut db, payload, None, i % 10 == 0)
                .unwrap();
        }
        println!("create 10k items: {:?}", start.elapsed());

        let start = Instant::now();
        let vault = vault::unlock(&db, PASSWORD).unwrap();
        println!("unlock + decrypt 10k (test KDF): {:?}", start.elapsed());
        assert_eq!(vault.items.len(), 10_000);

        let start = Instant::now();
        let listed = vault.list_items(&all());
        println!("list all (sorted): {:?}", start.elapsed());
        assert_eq!(listed.len(), 10_000);

        for query in ["user42", "site 99", "recovery 1234", "nothing-matches-this"] {
            let filter = ItemFilter {
                query: Some(query.into()),
                ..all()
            };
            let start = Instant::now();
            let found = vault.list_items(&filter);
            println!(
                "search {query:?}: {} hits in {:?}",
                found.len(),
                start.elapsed()
            );
        }
    }
}
