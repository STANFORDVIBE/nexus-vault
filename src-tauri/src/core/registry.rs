//! List of known vault files (`vaults.json` in the app data dir).
//!
//! The registry holds only names and paths, never secrets. The first launch registers
//! the default vault (`<data dir>/vault.db`), so existing installs keep working.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::core::error::{AppError, AppResult};
use crate::core::time::now_millis;
use crate::crypto::random;

pub const REGISTRY_FILE: &str = "vaults.json";
pub const DEFAULT_VAULT_FILE: &str = "vault.db";
pub const DEFAULT_VAULT_NAME: &str = "Основное";
/// Extension of vault files created by "new vault".
pub const VAULT_EXTENSION: &str = "nexusvault";
pub const MAX_VAULT_NAME: usize = 64;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VaultEntry {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    #[serde(default)]
    pub last_opened_at: Option<i64>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegistryFile {
    active: String,
    vaults: Vec<VaultEntry>,
}

pub struct Registry {
    file: PathBuf,
    data_dir: PathBuf,
    pub vaults: Vec<VaultEntry>,
    pub active: String,
}

pub fn validate_name(name: &str) -> AppResult<String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > MAX_VAULT_NAME {
        return Err(AppError::Validation("vault_name"));
    }
    Ok(name.to_string())
}

/// File-system-safe version of a vault name for its default file name.
fn file_stem_for(name: &str) -> String {
    let stem: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let stem = stem.trim().trim_end_matches('.').to_string();
    if stem.is_empty() {
        "vault".into()
    } else {
        stem
    }
}

impl Registry {
    /// Loads the registry, creating the default entry if the file is missing or broken.
    pub fn load(data_dir: &Path) -> Registry {
        let file = data_dir.join(REGISTRY_FILE);
        let stored: Option<RegistryFile> = fs::read(&file)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok());
        let mut registry = Registry {
            file,
            data_dir: data_dir.to_path_buf(),
            vaults: Vec::new(),
            active: String::new(),
        };
        if let Some(stored) = stored.filter(|s| !s.vaults.is_empty()) {
            registry.vaults = stored.vaults;
            registry.active = stored.active;
        }
        if registry.vaults.is_empty() {
            registry.vaults.push(VaultEntry {
                id: random::new_id().unwrap_or_else(|_| "default".into()),
                name: DEFAULT_VAULT_NAME.into(),
                path: data_dir.join(DEFAULT_VAULT_FILE),
                last_opened_at: None,
            });
        }
        if registry.get(&registry.active.clone()).is_none() {
            registry.active = registry.vaults[0].id.clone();
        }
        registry
    }

    pub fn save(&self) -> AppResult<()> {
        fs::create_dir_all(&self.data_dir)?;
        let json = serde_json::to_vec_pretty(&RegistryFile {
            active: self.active.clone(),
            vaults: self.vaults.clone(),
        })
        .map_err(|_| AppError::Internal)?;
        let tmp = self.file.with_extension("json.tmp");
        {
            let mut f = fs::File::create(&tmp)?;
            f.write_all(&json)?;
            f.sync_all()?;
        }
        fs::rename(&tmp, &self.file)?;
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&VaultEntry> {
        self.vaults.iter().find(|v| v.id == id)
    }

    pub fn active_entry(&self) -> &VaultEntry {
        // `load` guarantees the active id exists and the list is never empty
        self.get(&self.active).unwrap_or(&self.vaults[0])
    }

    pub fn find_by_path(&self, path: &Path) -> Option<&VaultEntry> {
        let wanted = normalize(path);
        self.vaults.iter().find(|v| normalize(&v.path) == wanted)
    }

    /// Where "new vault" puts files when the user does not pick a folder.
    pub fn default_folder(&self) -> PathBuf {
        self.data_dir.join("vaults")
    }

    /// A free `<folder>/<name>.nexusvault` path (adds " 2", " 3"… on collisions).
    pub fn new_vault_path(&self, folder: &Path, name: &str) -> PathBuf {
        let stem = file_stem_for(name);
        let mut n = 1;
        loop {
            let file = if n == 1 {
                format!("{stem}.{VAULT_EXTENSION}")
            } else {
                format!("{stem} {n}.{VAULT_EXTENSION}")
            };
            let path = folder.join(file);
            if !path.exists() && self.find_by_path(&path).is_none() {
                return path;
            }
            n += 1;
        }
    }

    pub fn add(&mut self, name: String, path: PathBuf) -> AppResult<String> {
        let id = random::new_id()?;
        self.vaults.push(VaultEntry {
            id: id.clone(),
            name,
            path,
            last_opened_at: None,
        });
        Ok(id)
    }

    pub fn rename(&mut self, id: &str, name: String) -> AppResult<()> {
        let entry = self
            .vaults
            .iter_mut()
            .find(|v| v.id == id)
            .ok_or(AppError::NotFound)?;
        entry.name = name;
        Ok(())
    }

    /// Removes a vault from the list (the file itself is left alone).
    pub fn forget(&mut self, id: &str) -> AppResult<()> {
        if self.get(id).is_none() {
            return Err(AppError::NotFound);
        }
        if self.vaults.len() == 1 {
            return Err(AppError::Validation("last_vault"));
        }
        self.vaults.retain(|v| v.id != id);
        if self.active == id {
            self.active = self.vaults[0].id.clone();
        }
        Ok(())
    }

    pub fn set_active(&mut self, id: &str) -> AppResult<()> {
        if self.get(id).is_none() {
            return Err(AppError::NotFound);
        }
        self.active = id.to_string();
        Ok(())
    }

    pub fn mark_opened(&mut self) {
        let active = self.active.clone();
        if let Some(entry) = self.vaults.iter_mut().find(|v| v.id == active) {
            entry.last_opened_at = Some(now_millis());
        }
    }
}

/// Case-insensitive comparison key (Windows paths), without `canonicalize`, which
/// fails for files that do not exist yet.
fn normalize(path: &Path) -> String {
    path.to_string_lossy().replace('/', "\\").to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_load_registers_the_default_vault() {
        let dir = tempfile::tempdir().unwrap();
        let registry = Registry::load(dir.path());
        assert_eq!(registry.vaults.len(), 1);
        let active = registry.active_entry();
        assert_eq!(active.name, DEFAULT_VAULT_NAME);
        assert_eq!(active.path, dir.path().join(DEFAULT_VAULT_FILE));
    }

    #[test]
    fn add_switch_forget_and_persist() {
        let dir = tempfile::tempdir().unwrap();
        let mut registry = Registry::load(dir.path());
        let first = registry.active.clone();
        let path = registry.new_vault_path(&registry.default_folder(), "Работа");
        assert!(path.ends_with("Работа.nexusvault"));
        let work = registry.add("Работа".into(), path).unwrap();
        registry.set_active(&work).unwrap();
        registry.save().unwrap();

        let mut reloaded = Registry::load(dir.path());
        assert_eq!(reloaded.vaults.len(), 2);
        assert_eq!(reloaded.active, work);

        reloaded.forget(&work).unwrap();
        assert_eq!(reloaded.active, first);
        assert!(matches!(
            reloaded.forget(&first),
            Err(AppError::Validation("last_vault"))
        ));
    }

    #[test]
    fn new_paths_avoid_collisions_and_bad_characters() {
        let dir = tempfile::tempdir().unwrap();
        let mut registry = Registry::load(dir.path());
        let folder = dir.path().to_path_buf();
        let a = registry.new_vault_path(&folder, "a/b:c");
        assert_eq!(a.file_name().unwrap(), "a_b_c.nexusvault");
        registry.add("x".into(), a.clone()).unwrap();
        let b = registry.new_vault_path(&folder, "a/b:c");
        assert_eq!(b.file_name().unwrap(), "a_b_c 2.nexusvault");
    }

    #[test]
    fn rejects_empty_or_long_names() {
        assert!(validate_name("   ").is_err());
        assert!(validate_name(&"я".repeat(65)).is_err());
        assert_eq!(validate_name("  Дом ").unwrap(), "Дом");
    }
}
