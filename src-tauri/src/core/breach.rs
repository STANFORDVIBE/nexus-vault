//! Leaked password check via Have I Been Pwned (k-anonymity range API).
//!
//! Only the first 5 hex characters of each password's SHA-1 leave the computer; the
//! service answers with every known hash starting with them (padded with decoys) and
//! the match happens here. Neither the password nor its full hash is ever sent.
//! This module is the offline part; the requests are made in `commands::breach`.

use std::collections::HashMap;

use serde::Serialize;
use sha1::{Digest, Sha1};
use zeroize::Zeroizing;

use crate::core::items::{ItemSummary, summary};
use crate::core::model::ItemPayload;
use crate::core::vault::UnlockedVault;

/// Hash suffixes to look for under one 5-character prefix, with the items using them.
pub type Lookups = HashMap<String, Vec<(Zeroizing<String>, Vec<String>)>>;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BreachedEntry {
    pub item: ItemSummary,
    /// How many times the password appears in known leaks.
    pub count: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BreachReport {
    /// Logins with a password that were checked.
    pub checked: usize,
    pub breached: Vec<BreachedEntry>,
}

/// Groups the passwords of active logins by hash prefix (one request per prefix).
pub fn lookups(vault: &UnlockedVault) -> (usize, Lookups) {
    let mut by_hash: HashMap<[u8; 20], Vec<String>> = HashMap::new();
    let mut checked = 0;
    for item in vault.items.values().filter(|i| i.deleted_at.is_none()) {
        if let ItemPayload::Login(p) = &item.payload
            && let Some(password) = &p.password
        {
            checked += 1;
            let digest: [u8; 20] = Sha1::digest(password.as_bytes()).into();
            by_hash.entry(digest).or_default().push(item.id.clone());
        }
    }
    let mut out: Lookups = HashMap::new();
    for (digest, ids) in by_hash {
        let mut hex = Zeroizing::new(String::with_capacity(40));
        for byte in digest {
            hex.push_str(&format!("{byte:02X}"));
        }
        let (prefix, suffix) = hex.split_at(5);
        out.entry(prefix.to_string())
            .or_default()
            .push((Zeroizing::new(suffix.to_string()), ids));
    }
    (checked, out)
}

/// Parses a range response (`SUFFIX:COUNT` per line). Padding decoys have count 0.
pub fn parse_range(body: &str) -> HashMap<String, u64> {
    body.lines()
        .filter_map(|line| {
            let (suffix, count) = line.trim().split_once(':')?;
            let count: u64 = count.trim().parse().ok()?;
            (count > 0).then(|| (suffix.to_ascii_uppercase(), count))
        })
        .collect()
}

/// Item ids whose password was found, with the leak count.
pub fn matches(
    wanted: &[(Zeroizing<String>, Vec<String>)],
    found: &HashMap<String, u64>,
) -> Vec<(String, u64)> {
    wanted
        .iter()
        .filter_map(|(suffix, ids)| found.get(suffix.as_str()).map(|count| (ids, *count)))
        .flat_map(|(ids, count)| ids.iter().map(move |id| (id.clone(), count)))
        .collect()
}

/// The final report for items that still exist, most leaked first.
pub fn report(vault: &UnlockedVault, checked: usize, hits: Vec<(String, u64)>) -> BreachReport {
    let mut breached: Vec<BreachedEntry> = hits
        .into_iter()
        .filter_map(|(id, count)| {
            let item = vault.items.get(&id)?;
            Some(BreachedEntry {
                item: summary(item),
                count,
            })
        })
        .collect();
    breached.sort_by(|a, b| b.count.cmp(&a.count).then(a.item.title.cmp(&b.item.title)));
    BreachReport { checked, breached }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::model::LoginPayload;
    use crate::core::vault;
    use crate::crypto::kdf::KdfParams;
    use crate::database::Database;

    #[test]
    fn finds_leaked_passwords_from_a_range_response() {
        let mut db = Database::open_in_memory().unwrap();
        let mut v = vault::create(&mut db, "correct horse battery", KdfParams::TEST).unwrap();
        let login = |title: &str, password: &str| {
            ItemPayload::Login(LoginPayload {
                title: title.into(),
                password: Some(password.into()),
                ..Default::default()
            })
        };
        let a = v
            .create_item(&mut db, login("A", "password"), None, false)
            .unwrap();
        let b = v
            .create_item(&mut db, login("B", "password"), None, false)
            .unwrap();
        v.create_item(&mut db, login("C", "x9$Lq!vT2#pW"), None, false)
            .unwrap();

        let (checked, lookups) = lookups(&v);
        assert_eq!(checked, 3);
        // SHA-1("password") = 5BAA61E4C9B93F3F0682250B6CF8331B7EE68FD8
        let wanted = &lookups["5BAA6"];
        assert_eq!(wanted.len(), 1, "the same password is looked up once");
        assert_eq!(wanted[0].0.as_str(), "1E4C9B93F3F0682250B6CF8331B7EE68FD8");

        let body = "0018A45C4D1DEF81644B54AB7F969B88D65:0\r\n\
                    1E4C9B93F3F0682250B6CF8331B7EE68FD8:9659365\r\n";
        let found = parse_range(body);
        assert_eq!(found.len(), 1, "padding decoys are ignored");
        let report = report(&v, checked, matches(wanted, &found));
        assert_eq!(report.breached.len(), 2);
        assert!(report.breached.iter().all(|e| e.count == 9_659_365));
        let ids: Vec<&str> = report.breached.iter().map(|e| e.item.id.as_str()).collect();
        assert!(ids.contains(&a.as_str()) && ids.contains(&b.as_str()));
    }
}
