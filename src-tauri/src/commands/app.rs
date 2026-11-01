use serde::Serialize;

use crate::core::error::AppResult;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct AppInfo {
    pub name: &'static str,
    pub version: &'static str,
}

fn app_info() -> AppInfo {
    AppInfo {
        name: "Nexus Vault",
        version: env!("CARGO_PKG_VERSION"),
    }
}

#[tauri::command]
pub fn get_app_info() -> AppResult<AppInfo> {
    Ok(app_info())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_crate_version() {
        let info = app_info();
        assert_eq!(info.name, "Nexus Vault");
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn serializes_for_the_frontend() {
        let json = serde_json::to_value(app_info()).unwrap();
        assert_eq!(json["name"], "Nexus Vault");
        assert!(json["version"].is_string());
    }
}
