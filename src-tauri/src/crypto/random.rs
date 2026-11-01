use crate::core::error::{AppError, AppResult};

/// Fills `buf` from the operating system CSPRNG.
pub fn fill(buf: &mut [u8]) -> AppResult<()> {
    getrandom::fill(buf).map_err(|_| AppError::Internal)
}

pub fn bytes<const N: usize>() -> AppResult<[u8; N]> {
    let mut buf = [0u8; N];
    fill(&mut buf)?;
    Ok(buf)
}

/// Random UUIDv4 for row identifiers: carries no information about the content.
pub fn new_id() -> AppResult<String> {
    let raw: [u8; 16] = bytes()?;
    Ok(uuid::Builder::from_random_bytes(raw)
        .into_uuid()
        .hyphenated()
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn ids_are_unique_uuid_v4() {
        let ids: HashSet<String> = (0..1000).map(|_| new_id().unwrap()).collect();
        assert_eq!(ids.len(), 1000);
        let id = ids.iter().next().unwrap();
        assert_eq!(uuid::Uuid::parse_str(id).unwrap().get_version_num(), 4);
    }
}
