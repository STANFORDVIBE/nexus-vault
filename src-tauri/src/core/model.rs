//! Vault item types and validation. Payload structs are wiped from memory on drop.

use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::core::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemType {
    Login,
    SecureNote,
}

impl ItemType {
    pub fn as_str(self) -> &'static str {
        match self {
            ItemType::Login => "login",
            ItemType::SecureNote => "secure_note",
        }
    }

    pub fn parse(value: &str) -> AppResult<Self> {
        match value {
            "login" => Ok(ItemType::Login),
            "secure_note" => Ok(ItemType::SecureNote),
            _ => Err(AppError::UnsupportedVersion),
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize, Zeroize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LoginPayload {
    pub title: String,
    #[serde(default)]
    pub website: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Clone, Default, Serialize, Deserialize, Zeroize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecureNotePayload {
    pub title: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Decrypted item content, tagged with its type on the wire:
/// `{ "type": "login", "payload": { ... } }`.
#[derive(Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum ItemPayload {
    Login(LoginPayload),
    SecureNote(SecureNotePayload),
}

const MAX_TITLE: usize = 200;
const MAX_SHORT_FIELD: usize = 512;
const MAX_URL: usize = 2048;
const MAX_PASSWORD: usize = 4096;
const MAX_NOTES: usize = 100_000;
const MAX_NOTE_CONTENT: usize = 1_000_000;
const MAX_TAGS: usize = 32;
const MAX_TAG: usize = 64;

fn check_len(value: &str, max: usize, field: &'static str) -> AppResult<()> {
    if value.chars().count() > max {
        return Err(AppError::Validation(field));
    }
    Ok(())
}

fn check_optional(value: &Option<String>, max: usize, field: &'static str) -> AppResult<()> {
    value
        .as_deref()
        .map_or(Ok(()), |v| check_len(v, max, field))
}

/// Trims text fields, turns empty optionals into `None` and normalizes tags.
fn clean_optional(value: &mut Option<String>) {
    if let Some(v) = value {
        let trimmed = v.trim();
        if trimmed.is_empty() {
            value.zeroize();
            *value = None;
        } else if trimmed.len() != v.len() {
            *v = trimmed.to_string();
        }
    }
}

fn clean_tags(tags: &mut Vec<String>) -> AppResult<()> {
    let mut cleaned: Vec<String> = Vec::with_capacity(tags.len());
    for tag in tags.iter() {
        let tag = tag.trim();
        if tag.is_empty() {
            continue;
        }
        check_len(tag, MAX_TAG, "tags")?;
        if !cleaned.iter().any(|t| t.eq_ignore_ascii_case(tag)) {
            cleaned.push(tag.to_string());
        }
    }
    if cleaned.len() > MAX_TAGS {
        return Err(AppError::Validation("tags"));
    }
    tags.zeroize();
    *tags = cleaned;
    Ok(())
}

fn clean_title(title: &mut String) -> AppResult<()> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation("title"));
    }
    check_len(trimmed, MAX_TITLE, "title")?;
    if trimmed.len() != title.len() {
        *title = trimmed.to_string();
    }
    Ok(())
}

impl ItemPayload {
    pub fn item_type(&self) -> ItemType {
        match self {
            ItemPayload::Login(_) => ItemType::Login,
            ItemPayload::SecureNote(_) => ItemType::SecureNote,
        }
    }

    pub fn title(&self) -> &str {
        match self {
            ItemPayload::Login(p) => &p.title,
            ItemPayload::SecureNote(p) => &p.title,
        }
    }

    pub fn tags(&self) -> &[String] {
        match self {
            ItemPayload::Login(p) => &p.tags,
            ItemPayload::SecureNote(p) => &p.tags,
        }
    }

    /// Backend-side validation: the frontend checks too, but this is authoritative.
    pub fn validate_and_normalize(&mut self) -> AppResult<()> {
        match self {
            ItemPayload::Login(p) => {
                clean_title(&mut p.title)?;
                clean_optional(&mut p.website);
                clean_optional(&mut p.username);
                clean_optional(&mut p.email);
                clean_optional(&mut p.notes);
                // passwords are kept verbatim (leading/trailing spaces may be intentional)
                if p.password.as_deref() == Some("") {
                    p.password = None;
                }
                check_optional(&p.website, MAX_URL, "website")?;
                check_optional(&p.username, MAX_SHORT_FIELD, "username")?;
                check_optional(&p.email, MAX_SHORT_FIELD, "email")?;
                check_optional(&p.password, MAX_PASSWORD, "password")?;
                check_optional(&p.notes, MAX_NOTES, "notes")?;
                clean_tags(&mut p.tags)
            }
            ItemPayload::SecureNote(p) => {
                clean_title(&mut p.title)?;
                check_len(&p.content, MAX_NOTE_CONTENT, "content")?;
                clean_tags(&mut p.tags)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn login(title: &str) -> ItemPayload {
        ItemPayload::Login(LoginPayload {
            title: title.into(),
            ..Default::default()
        })
    }

    #[test]
    fn deserializes_tagged_payload() {
        let json = r#"{"type":"login","payload":{"title":"GitHub","username":"me","tags":[]}}"#;
        let payload: ItemPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.item_type(), ItemType::Login);
        assert_eq!(payload.title(), "GitHub");
    }

    #[test]
    fn rejects_unknown_fields() {
        let json = r#"{"type":"login","payload":{"title":"x","evil":1}}"#;
        assert!(serde_json::from_str::<ItemPayload>(json).is_err());
    }

    #[test]
    fn requires_title() {
        assert!(matches!(
            login("   ").validate_and_normalize(),
            Err(AppError::Validation("title"))
        ));
        assert!(login(&"x".repeat(201)).validate_and_normalize().is_err());
    }

    #[test]
    fn normalizes_fields_and_tags() {
        let mut payload = ItemPayload::Login(LoginPayload {
            title: "  GitHub ".into(),
            website: Some("   ".into()),
            password: Some(" keep spaces ".into()),
            tags: vec![" Work ".into(), "work".into(), "".into(), "dev".into()],
            ..Default::default()
        });
        payload.validate_and_normalize().unwrap();
        let ItemPayload::Login(p) = &payload else {
            unreachable!()
        };
        assert_eq!(p.title, "GitHub");
        assert_eq!(p.website, None);
        assert_eq!(p.password.as_deref(), Some(" keep spaces "));
        assert_eq!(p.tags, vec!["Work".to_string(), "dev".to_string()]);
    }

    #[test]
    fn limits_tag_count() {
        let mut payload = ItemPayload::SecureNote(SecureNotePayload {
            title: "t".into(),
            content: String::new(),
            tags: (0..33).map(|i| format!("tag{i}")).collect(),
        });
        assert!(payload.validate_and_normalize().is_err());
    }
}
