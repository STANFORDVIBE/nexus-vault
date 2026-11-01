use serde::{Serialize, Serializer};

/// Errors that may cross the IPC boundary.
///
/// Only a stable `code` and a generic message reach the frontend: internal details
/// (paths, SQL, crypto failures) are never serialized, so they cannot leak secrets.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("vault is not initialized")]
    VaultNotInitialized,
    #[error("vault already exists")]
    VaultAlreadyExists,
    #[error("vault is locked")]
    VaultLocked,
    #[error("invalid master password")]
    InvalidPassword,
    #[error("data integrity check failed")]
    Integrity,
    #[error("not found")]
    NotFound,
    #[error("validation failed: {0}")]
    Validation(&'static str),
    #[error("unsupported format version")]
    UnsupportedVersion,
    #[error("invalid backup file")]
    InvalidBackup,
    #[error("operation cancelled")]
    Cancelled,
    #[error("database error")]
    Database(#[source] rusqlite::Error),
    #[error("io error")]
    Io(#[source] std::io::Error),
    #[error("internal error")]
    Internal,
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            AppError::VaultNotInitialized => "vault_not_initialized",
            AppError::VaultAlreadyExists => "vault_already_exists",
            AppError::VaultLocked => "vault_locked",
            AppError::InvalidPassword => "invalid_password",
            AppError::Integrity => "integrity",
            AppError::NotFound => "not_found",
            AppError::Validation(_) => "validation",
            AppError::UnsupportedVersion => "unsupported_version",
            AppError::InvalidBackup => "invalid_backup",
            AppError::Cancelled => "cancelled",
            AppError::Database(_) => "database",
            AppError::Io(_) => "io",
            AppError::Internal => "internal",
        }
    }

    /// Message safe to show in the UI. Validation errors carry a static field code;
    /// everything else gets a generic text so internals never leak.
    fn public_message(&self) -> String {
        match self {
            AppError::Validation(field) => format!("validation failed: {field}"),
            AppError::Database(_) => "database error".into(),
            AppError::Io(_) => "io error".into(),
            other => other.to_string(),
        }
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(error: rusqlite::Error) -> Self {
        AppError::Database(error)
    }
}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        AppError::Io(error)
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("AppError", 2)?;
        s.serialize_field("code", self.code())?;
        s.serialize_field("message", &self.public_message())?;
        s.end()
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_only_code_and_message() {
        let json = serde_json::to_value(AppError::Internal).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "code": "internal", "message": "internal error" })
        );
    }

    #[test]
    fn database_errors_do_not_leak_details() {
        let error = AppError::from(rusqlite::Error::InvalidColumnName("secret_column".into()));
        let json = serde_json::to_string(&error).unwrap();
        assert!(!json.contains("secret_column"));
        assert!(json.contains("\"database\""));
    }
}
