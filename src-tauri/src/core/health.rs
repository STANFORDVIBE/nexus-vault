//! Password health: weak, reused and old passwords. Everything is computed in memory
//! from the unlocked vault; the result lists items only, never the passwords.

use std::collections::HashMap;

use serde::Serialize;

use crate::core::items::{ItemSummary, summary};
use crate::core::model::ItemPayload;
use crate::core::vault::{UnlockedVault, VaultItem};

/// A password not changed for this long is reported as old.
pub const OLD_AFTER_DAYS: i64 = 365;
const DAY_MS: i64 = 24 * 60 * 60 * 1000;

const COMMON: &[&str] = &[
    "password",
    "qwerty",
    "123456",
    "111111",
    "admin",
    "letmein",
    "welcome",
    "iloveyou",
    "пароль",
    "йцукен",
];

/// 0..=4, same scale and thresholds as the strength meter in the UI.
pub fn strength_level(password: &str) -> u8 {
    if password.is_empty() {
        return 0;
    }
    let chars: Vec<char> = password.chars().collect();
    let has = |f: fn(&char) -> bool| chars.iter().any(f);
    let mut pool = 0u32;
    if has(|c| c.is_ascii_lowercase()) {
        pool += 26;
    }
    if has(|c| c.is_ascii_uppercase()) {
        pool += 26;
    }
    if has(|c| c.is_ascii_digit()) {
        pool += 10;
    }
    if has(|c| matches!(c, 'а'..='я' | 'ё')) {
        pool += 33;
    }
    if has(|c| matches!(c, 'А'..='Я' | 'Ё')) {
        pool += 33;
    }
    if has(|c| !c.is_ascii_alphanumeric() && !matches!(c, 'а'..='я' | 'ё' | 'А'..='Я' | 'Ё'))
    {
        pool += 33;
    }
    let mut distinct = chars.clone();
    distinct.sort_unstable();
    distinct.dedup();
    let effective = distinct.len() as f64 + (chars.len() - distinct.len()) as f64 * 0.25;
    let mut bits = effective * f64::from(pool.max(1)).log2();
    let lower = password.to_lowercase();
    if COMMON.iter().any(|w| lower.contains(w)) {
        bits *= 0.5;
    }
    if chars.len() > 1 && chars.iter().all(|c| *c == chars[0]) {
        bits = bits.min(8.0);
    }
    match (chars.len() < 10, bits) {
        (true, b) if b < 28.0 => 0,
        (true, _) => 1,
        (false, b) if b < 45.0 => 1,
        (false, b) if b < 60.0 => 2,
        (false, b) if b < 80.0 => 3,
        _ => 4,
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeakEntry {
    pub item: ItemSummary,
    pub level: u8,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OldEntry {
    pub item: ItemSummary,
    pub changed_at: i64,
    pub age_days: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordHealth {
    /// Logins with a password (the ones checked).
    pub checked: usize,
    /// Logins with no problem at all.
    pub healthy: usize,
    pub weak: Vec<WeakEntry>,
    /// Each group shares one password.
    pub reused: Vec<Vec<ItemSummary>>,
    pub old: Vec<OldEntry>,
    pub old_after_days: i64,
}

fn password_of(item: &VaultItem) -> Option<&str> {
    match &item.payload {
        ItemPayload::Login(p) if item.deleted_at.is_none() => {
            p.password.as_deref().filter(|pw| !pw.is_empty())
        }
        _ => None,
    }
}

pub fn check(vault: &UnlockedVault, now: i64) -> PasswordHealth {
    let mut logins: Vec<(&VaultItem, &str)> = vault
        .items
        .values()
        .filter_map(|i| password_of(i).map(|pw| (i, pw)))
        .collect();
    logins.sort_by_key(|(i, _)| i.payload.title().to_lowercase());

    let mut problems: HashMap<&str, ()> = HashMap::new();

    let mut weak = Vec::new();
    for (item, pw) in &logins {
        let level = strength_level(pw);
        if level < 2 {
            weak.push(WeakEntry {
                item: summary(item),
                level,
            });
            problems.insert(item.id.as_str(), ());
        }
    }
    weak.sort_by_key(|w| w.level);

    let mut groups: HashMap<&str, Vec<&VaultItem>> = HashMap::new();
    for (item, pw) in &logins {
        groups.entry(pw).or_default().push(item);
    }
    let mut reused: Vec<Vec<ItemSummary>> = groups
        .into_values()
        .filter(|g| g.len() > 1)
        .map(|g| {
            for item in &g {
                problems.insert(item.id.as_str(), ());
            }
            g.into_iter().map(summary).collect()
        })
        .collect();
    reused.sort_by(|a, b| {
        b.len()
            .cmp(&a.len())
            .then_with(|| a[0].title.cmp(&b[0].title))
    });

    let mut old = Vec::new();
    for (item, _) in &logins {
        let changed_at = match &item.payload {
            ItemPayload::Login(p) => p.password_changed_at.unwrap_or(item.created_at),
            _ => item.created_at,
        };
        let age_days = (now - changed_at) / DAY_MS;
        if age_days >= OLD_AFTER_DAYS {
            old.push(OldEntry {
                item: summary(item),
                changed_at,
                age_days,
            });
            problems.insert(item.id.as_str(), ());
        }
    }
    old.sort_by_key(|o| std::cmp::Reverse(o.age_days));

    PasswordHealth {
        checked: logins.len(),
        healthy: logins.len() - problems.len(),
        weak,
        reused,
        old,
        old_after_days: OLD_AFTER_DAYS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::model::LoginPayload;
    use crate::core::vault;
    use crate::crypto::kdf::KdfParams;
    use crate::database::Database;

    #[test]
    fn strength_matches_the_ui_scale() {
        assert_eq!(strength_level(""), 0);
        assert_eq!(strength_level("123456"), 0);
        assert_eq!(strength_level("aaaaaaaaaaaaaaa"), 1);
        assert!(strength_level("password12345") <= 1);
        assert!(strength_level("Kx9#mP2$vL7@qR4!") >= 3);
        assert!(strength_level("лес-река-камень-облако") >= 3);
    }

    #[test]
    fn finds_weak_reused_and_old_passwords() {
        let mut db = Database::open_in_memory().unwrap();
        let mut v = vault::create(&mut db, "correct horse battery", KdfParams::TEST).unwrap();
        let mut add = |v: &mut UnlockedVault, title: &str, pw: &str| {
            v.create_item(
                &mut db,
                ItemPayload::Login(LoginPayload {
                    title: title.into(),
                    password: Some(pw.into()),
                    ..Default::default()
                }),
                None,
                false,
            )
            .unwrap()
        };
        add(&mut v, "Weak", "qwerty");
        add(&mut v, "A", "Shared-Strong-Passphrase-42!");
        add(&mut v, "B", "Shared-Strong-Passphrase-42!");
        let fine = add(&mut v, "Fine", "Unique-Strong-Passphrase-77?");

        let now = crate::core::time::now_millis();
        let report = check(&v, now);
        assert_eq!(report.checked, 4);
        assert_eq!(report.weak.len(), 1);
        assert_eq!(report.weak[0].item.title, "Weak");
        assert_eq!(report.reused.len(), 1);
        assert_eq!(report.reused[0].len(), 2);
        assert!(report.old.is_empty());
        assert_eq!(report.healthy, 1);

        let later = check(&v, now + 400 * DAY_MS);
        assert_eq!(later.old.len(), 4);
        assert_eq!(later.healthy, 0);
        assert!(
            later
                .old
                .iter()
                .any(|o| o.item.id == fine && o.age_days >= 400)
        );

        let json = serde_json::to_string(&report).unwrap();
        assert!(!json.contains("qwerty") && !json.contains("Passphrase"));
    }
}
