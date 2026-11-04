//! Network part of the leaked password check (see `core::breach`). Runs only when the
//! user presses the button; sends 5 hex characters of a hash per request, nothing else.

use std::time::Duration;

use tauri::State;

use crate::commands::SharedState;
use crate::core::breach::{self, BreachReport};
use crate::core::error::{AppError, AppResult};

const RANGE_URL: &str = "https://api.pwnedpasswords.com/range/";
/// Parallel requests: fast enough for big vaults, polite to a free service.
const PARALLEL: usize = 6;

async fn fetch_range(client: &reqwest::Client, prefix: &str) -> AppResult<String> {
    let response = client
        .get(format!("{RANGE_URL}{prefix}"))
        // decoy entries, so the response size does not hint at the prefix
        .header("Add-Padding", "true")
        .send()
        .await
        .map_err(|_| AppError::Network)?;
    if !response.status().is_success() {
        return Err(AppError::Network);
    }
    response.text().await.map_err(|_| AppError::Network)
}

#[tauri::command]
pub async fn check_breaches(state: State<'_, SharedState>) -> AppResult<BreachReport> {
    let (checked, lookups) = {
        let mut inner = state.lock();
        inner.touch();
        let (_, vault) = inner.unlocked()?;
        breach::lookups(vault)
    };

    // same TLS stack as the updater (rustls + ring); installing twice is harmless
    let _ = rustls::crypto::ring::default_provider().install_default();
    let client = reqwest::Client::builder()
        .user_agent(concat!("nexus-vault/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|_| AppError::Network)?;

    let lookups: Vec<_> = lookups.into_iter().collect();
    let mut hits = Vec::new();
    for chunk in lookups.chunks(PARALLEL) {
        let tasks: Vec<_> = chunk
            .iter()
            .map(|(prefix, _)| {
                let client = client.clone();
                let prefix = prefix.clone();
                tauri::async_runtime::spawn(async move { fetch_range(&client, &prefix).await })
            })
            .collect();
        for ((_, wanted), task) in chunk.iter().zip(tasks) {
            let body = task.await.map_err(|_| AppError::Internal)??;
            hits.extend(breach::matches(wanted, &breach::parse_range(&body)));
        }
    }

    let mut inner = state.lock();
    let (_, vault) = inner.unlocked()?;
    Ok(breach::report(vault, checked, hits))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Talks to the real service. Run with: cargo test -- --ignored live_range_lookup
    #[test]
    #[ignore]
    fn live_range_lookup() {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let client = reqwest::Client::builder()
            .user_agent("nexus-vault-test")
            .build()
            .unwrap();
        let body = tauri::async_runtime::block_on(fetch_range(&client, "5BAA6")).unwrap();
        // SHA-1("password") is one of the best known leaked hashes
        let found = breach::parse_range(&body);
        assert!(found["1E4C9B93F3F0682250B6CF8331B7EE68FD8"] > 1_000_000);
    }
}
