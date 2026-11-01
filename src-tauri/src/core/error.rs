use serde::{Serialize, Serializer};

/// Errors that may cross the IPC boundary.
///
/// Only a stable `code` and a generic message reach the frontend: internal details
/// (paths, SQL, crypto failures) are never serialized, so they cannot leak secrets.
#[derive(Debug, thiserror::Error)]
#[allow(dead_code)] // variants are wired up by the Security Core (Phase 4)
pub enum AppError {
    #[error("internal error")]
    Internal,
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            AppError::Internal => "internal",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("AppError", 2)?;
        s.serialize_field("code", self.code())?;
        s.serialize_field("message", &self.to_string())?;
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
}
