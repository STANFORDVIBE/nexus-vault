//! Vault item types and validation. Payload structs are wiped from memory on drop.

use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::core::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemType {
    Login,
    SecureNote,
    Card,
    Identity,
}

impl ItemType {
    pub fn as_str(self) -> &'static str {
        match self {
            ItemType::Login => "login",
            ItemType::SecureNote => "secure_note",
            ItemType::Card => "card",
            ItemType::Identity => "identity",
        }
    }

    pub fn parse(value: &str) -> AppResult<Self> {
        match value {
            "login" => Ok(ItemType::Login),
            "secure_note" => Ok(ItemType::SecureNote),
            "card" => Ok(ItemType::Card),
            "identity" => Ok(ItemType::Identity),
            _ => Err(AppError::UnsupportedVersion),
        }
    }
}

/// User-defined extra field ("Секретный вопрос", "PIN"...). `hidden` fields are treated
/// like passwords: masked in the UI and copied by Rust only.
#[derive(Clone, Default, Serialize, Deserialize, Zeroize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CustomField {
    pub label: String,
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub hidden: bool,
}

/// A password the login used before, kept so a hasty change can be undone.
#[derive(Clone, Default, Serialize, Deserialize, Zeroize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OldPassword {
    pub password: String,
    /// When it was replaced.
    pub replaced_at: i64,
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
    #[serde(default)]
    pub fields: Vec<CustomField>,
    /// When the password last changed (set by the backend, for the password report).
    #[serde(default)]
    pub password_changed_at: Option<i64>,
    /// Previous passwords, newest first. Maintained by the backend only.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub password_history: Vec<OldPassword>,
}

#[derive(Clone, Default, Serialize, Deserialize, Zeroize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CardPayload {
    pub title: String,
    #[serde(default)]
    pub cardholder: Option<String>,
    /// Digits only (spaces are stripped on save).
    #[serde(default)]
    pub number: Option<String>,
    /// `MM/YY`.
    #[serde(default)]
    pub expiry: Option<String>,
    #[serde(default)]
    pub cvv: Option<String>,
    #[serde(default)]
    pub pin: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub fields: Vec<CustomField>,
}

/// A document: passport, driving licence, SNILS, insurance policy...
#[derive(Clone, Default, Serialize, Deserialize, Zeroize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IdentityPayload {
    pub title: String,
    /// Free text from a preset list in the UI (`passport`, `driver_license`, ...).
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub full_name: Option<String>,
    #[serde(default)]
    pub number: Option<String>,
    #[serde(default)]
    pub issued_by: Option<String>,
    #[serde(default)]
    pub issue_date: Option<String>,
    #[serde(default)]
    pub expiry_date: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub fields: Vec<CustomField>,
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
    Card(CardPayload),
    Identity(IdentityPayload),
}

const MAX_TITLE: usize = 200;
const MAX_SHORT_FIELD: usize = 512;
const MAX_URL: usize = 2048;
const MAX_PASSWORD: usize = 4096;
const MAX_NOTES: usize = 100_000;
const MAX_NOTE_CONTENT: usize = 1_000_000;
const MAX_TAGS: usize = 32;
const MAX_TAG: usize = 64;
const MAX_FIELDS: usize = 50;
const MAX_FIELD_LABEL: usize = 100;
const MAX_FIELD_VALUE: usize = 10_000;
const MAX_DATE: usize = 32;

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

/// Drops fully empty rows, trims labels; a value without a label gets a default one.
fn clean_fields(fields: &mut Vec<CustomField>) -> AppResult<()> {
    fields.retain(|f| !(f.label.trim().is_empty() && f.value.is_empty()));
    if fields.len() > MAX_FIELDS {
        return Err(AppError::Validation("fields"));
    }
    for field in fields.iter_mut() {
        let label = field.label.trim();
        let label = if label.is_empty() { "Поле" } else { label };
        check_len(label, MAX_FIELD_LABEL, "fields")?;
        check_len(&field.value, MAX_FIELD_VALUE, "fields")?;
        if label.len() != field.label.len() {
            field.label = label.to_string();
        }
    }
    Ok(())
}

/// Keeps only digits; `None` stays `None`. Errors if anything else is present.
fn clean_digits(
    value: &mut Option<String>,
    min: usize,
    max: usize,
    field: &'static str,
) -> AppResult<()> {
    clean_optional(value);
    if let Some(v) = value {
        let digits: String = v.chars().filter(|c| !matches!(c, ' ' | '-')).collect();
        if !digits.chars().all(|c| c.is_ascii_digit()) || !(min..=max).contains(&digits.len()) {
            return Err(AppError::Validation(field));
        }
        if digits.len() != v.len() {
            v.zeroize();
            *v = digits;
        }
    }
    Ok(())
}

/// `MM/YY` or `MM/YYYY` (stored as `MM/YY`).
fn clean_expiry(value: &mut Option<String>) -> AppResult<()> {
    clean_optional(value);
    if let Some(v) = value {
        let (mm, yy) = v.split_once('/').ok_or(AppError::Validation("expiry"))?;
        let (mm, yy) = (mm.trim(), yy.trim());
        let month: u32 = mm.parse().map_err(|_| AppError::Validation("expiry"))?;
        let year_ok = (yy.len() == 2 || yy.len() == 4) && yy.chars().all(|c| c.is_ascii_digit());
        if !(1..=12).contains(&month) || !year_ok {
            return Err(AppError::Validation("expiry"));
        }
        *v = format!("{month:02}/{}", &yy[yy.len() - 2..]);
    }
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
            ItemPayload::Card(_) => ItemType::Card,
            ItemPayload::Identity(_) => ItemType::Identity,
        }
    }

    pub fn title(&self) -> &str {
        match self {
            ItemPayload::Login(p) => &p.title,
            ItemPayload::SecureNote(p) => &p.title,
            ItemPayload::Card(p) => &p.title,
            ItemPayload::Identity(p) => &p.title,
        }
    }

    pub fn tags(&self) -> &[String] {
        match self {
            ItemPayload::Login(p) => &p.tags,
            ItemPayload::SecureNote(p) => &p.tags,
            ItemPayload::Card(p) => &p.tags,
            ItemPayload::Identity(p) => &p.tags,
        }
    }

    pub fn fields(&self) -> &[CustomField] {
        match self {
            ItemPayload::Login(p) => &p.fields,
            ItemPayload::SecureNote(_) => &[],
            ItemPayload::Card(p) => &p.fields,
            ItemPayload::Identity(p) => &p.fields,
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
                clean_fields(&mut p.fields)?;
                clean_tags(&mut p.tags)
            }
            ItemPayload::SecureNote(p) => {
                clean_title(&mut p.title)?;
                check_len(&p.content, MAX_NOTE_CONTENT, "content")?;
                clean_tags(&mut p.tags)
            }
            ItemPayload::Card(p) => {
                clean_title(&mut p.title)?;
                clean_optional(&mut p.cardholder);
                clean_optional(&mut p.notes);
                check_optional(&p.cardholder, MAX_SHORT_FIELD, "cardholder")?;
                check_optional(&p.notes, MAX_NOTES, "notes")?;
                clean_digits(&mut p.number, 12, 19, "card_number")?;
                clean_digits(&mut p.cvv, 3, 4, "cvv")?;
                clean_digits(&mut p.pin, 4, 12, "pin")?;
                clean_expiry(&mut p.expiry)?;
                clean_fields(&mut p.fields)?;
                clean_tags(&mut p.tags)
            }
            ItemPayload::Identity(p) => {
                clean_title(&mut p.title)?;
                for (value, max, name) in [
                    (&mut p.kind, MAX_TAG, "kind"),
                    (&mut p.full_name, MAX_SHORT_FIELD, "full_name"),
                    (&mut p.number, MAX_SHORT_FIELD, "doc_number"),
                    (&mut p.issued_by, MAX_SHORT_FIELD, "issued_by"),
                    (&mut p.issue_date, MAX_DATE, "date"),
                    (&mut p.expiry_date, MAX_DATE, "date"),
                    (&mut p.notes, MAX_NOTES, "notes"),
                ] {
                    clean_optional(value);
                    check_optional(value, max, name)?;
                }
                clean_fields(&mut p.fields)?;
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
    fn old_login_json_still_loads() {
        // saved before custom fields existed
        let json = r#"{"type":"login","payload":{"title":"x","password":"p","tags":[]}}"#;
        let payload: ItemPayload = serde_json::from_str(json).unwrap();
        assert!(payload.fields().is_empty());
    }

    #[test]
    fn card_fields_are_normalized_and_checked() {
        let mut card = ItemPayload::Card(CardPayload {
            title: "Visa".into(),
            number: Some("4111 1111-1111 1111".into()),
            expiry: Some("3/2029".into()),
            cvv: Some(" 123 ".into()),
            fields: vec![
                CustomField::default(),
                CustomField {
                    label: "  ".into(),
                    value: "x".into(),
                    hidden: true,
                },
            ],
            ..Default::default()
        });
        card.validate_and_normalize().unwrap();
        let ItemPayload::Card(p) = &card else {
            unreachable!()
        };
        assert_eq!(p.number.as_deref(), Some("4111111111111111"));
        assert_eq!(p.expiry.as_deref(), Some("03/29"));
        assert_eq!(p.cvv.as_deref(), Some("123"));
        assert_eq!(p.fields.len(), 1, "the empty row is dropped");
        assert_eq!(p.fields[0].label, "Поле");

        for (number, expiry, cvv) in [
            ("4111", "03/29", "123"),
            ("4111111111111111", "13/29", "123"),
            ("4111111111111111", "03/29", "12a"),
        ] {
            let mut bad = ItemPayload::Card(CardPayload {
                title: "x".into(),
                number: Some(number.into()),
                expiry: Some(expiry.into()),
                cvv: Some(cvv.into()),
                ..Default::default()
            });
            assert!(
                bad.validate_and_normalize().is_err(),
                "{number} {expiry} {cvv}"
            );
        }
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
