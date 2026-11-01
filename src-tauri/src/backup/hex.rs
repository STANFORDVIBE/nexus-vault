//! Minimal hex encoding for binary fields in the backup JSON.

pub fn encode(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(DIGITS[(b >> 4) as usize] as char);
        out.push(DIGITS[(b & 0x0f) as usize] as char);
    }
    out
}

pub fn decode(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    text.as_bytes()
        .chunks(2)
        .map(|pair| {
            let hi = (pair[0] as char).to_digit(16)?;
            let lo = (pair[1] as char).to_digit(16)?;
            Some((hi * 16 + lo) as u8)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let data = [0u8, 1, 0x7f, 0x80, 0xff];
        assert_eq!(encode(&data), "00017f80ff");
        assert_eq!(decode("00017f80ff").unwrap(), data);
        assert_eq!(decode("00017F80FF").unwrap(), data);
        assert!(decode("abc").is_none());
        assert!(decode("zz").is_none());
    }
}
