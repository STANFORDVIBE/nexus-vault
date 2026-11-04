//! One-time codes for two-factor authentication (TOTP, RFC 6238), like Google
//! Authenticator. A login stores either the bare base32 secret or a full
//! `otpauth://totp/...` link (the text inside the QR code a site shows).

use hmac::{Hmac, KeyInit, Mac};
use serde::Serialize;
use zeroize::Zeroizing;

use crate::core::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Algorithm {
    Sha1,
    Sha256,
    Sha512,
}

pub struct Totp {
    secret: Zeroizing<Vec<u8>>,
    digits: u32,
    period: u64,
    algorithm: Algorithm,
}

/// The current code and how long it stays valid.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TotpCode {
    pub code: String,
    pub period: u64,
    pub remaining: u64,
}

/// RFC 4648 base32 without padding; spaces, dashes and case are ignored.
fn decode_base32(text: &str) -> Option<Zeroizing<Vec<u8>>> {
    let mut out = Zeroizing::new(Vec::with_capacity(text.len() * 5 / 8));
    let (mut buffer, mut bits) = (0u64, 0u32);
    for c in text.chars().filter(|c| !matches!(c, ' ' | '-' | '=')) {
        let value = match c.to_ascii_uppercase() {
            c @ 'A'..='Z' => c as u64 - 'A' as u64,
            c @ '2'..='7' => c as u64 - '2' as u64 + 26,
            _ => return None,
        };
        buffer = (buffer << 5) | value;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    Some(out)
}

impl Totp {
    /// Parses a base32 secret or an `otpauth://totp/` link. Errors are `Validation("totp")`.
    pub fn parse(input: &str) -> AppResult<Totp> {
        let invalid = || AppError::Validation("totp");
        let input = input.trim();
        let mut totp = Totp {
            secret: Zeroizing::new(Vec::new()),
            digits: 6,
            period: 30,
            algorithm: Algorithm::Sha1,
        };
        let secret = if input.len() > 10 && input[..10].eq_ignore_ascii_case("otpauth://") {
            if !input[10..].to_ascii_lowercase().starts_with("totp/") {
                return Err(invalid());
            }
            let query = input.split_once('?').map_or("", |(_, q)| q);
            let mut secret = None;
            for (key, value) in query.split('&').filter_map(|p| p.split_once('=')) {
                match key.to_ascii_lowercase().as_str() {
                    "secret" => secret = Some(value),
                    "digits" => totp.digits = value.parse().map_err(|_| invalid())?,
                    "period" => totp.period = value.parse().map_err(|_| invalid())?,
                    "algorithm" => {
                        totp.algorithm = match value.to_ascii_uppercase().as_str() {
                            "SHA1" => Algorithm::Sha1,
                            "SHA256" => Algorithm::Sha256,
                            "SHA512" => Algorithm::Sha512,
                            _ => return Err(invalid()),
                        }
                    }
                    _ => {}
                }
            }
            secret.ok_or_else(invalid)?
        } else {
            input
        };
        totp.secret = decode_base32(secret).ok_or_else(invalid)?;
        if totp.secret.len() < 10
            || totp.secret.len() > 128
            || !(6..=8).contains(&totp.digits)
            || !(10..=120).contains(&totp.period)
        {
            return Err(invalid());
        }
        Ok(totp)
    }

    /// The code for the moment `unix_secs`.
    pub fn code_at(&self, unix_secs: u64) -> TotpCode {
        let counter = (unix_secs / self.period).to_be_bytes();
        let digest: Zeroizing<Vec<u8>> = Zeroizing::new(match self.algorithm {
            Algorithm::Sha1 => hmac_with!(sha1::Sha1, &self.secret, &counter),
            Algorithm::Sha256 => hmac_with!(sha2::Sha256, &self.secret, &counter),
            Algorithm::Sha512 => hmac_with!(sha2::Sha512, &self.secret, &counter),
        });
        // dynamic truncation (RFC 4226, 5.3)
        let offset = (digest[digest.len() - 1] & 0x0f) as usize;
        let value = u32::from_be_bytes([
            digest[offset] & 0x7f,
            digest[offset + 1],
            digest[offset + 2],
            digest[offset + 3],
        ]);
        let code = value % 10u32.pow(self.digits);
        TotpCode {
            code: format!("{code:0width$}", width = self.digits as usize),
            period: self.period,
            remaining: self.period - unix_secs % self.period,
        }
    }
}

/// HMAC of `message`; a macro because the `hmac` trait bounds are unwieldy generically.
macro_rules! hmac_with {
    ($hash:ty, $key:expr, $message:expr) => {{
        let mut mac =
            <Hmac<$hash> as KeyInit>::new_from_slice($key).expect("HMAC takes any key length");
        mac.update($message);
        mac.finalize().into_bytes().to_vec()
    }};
}
use hmac_with;

#[cfg(test)]
mod tests {
    use super::*;

    fn base32(bytes: &[u8]) -> String {
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
        let mut out = String::new();
        for chunk in bytes.chunks(5) {
            let mut buf = [0u8; 5];
            buf[..chunk.len()].copy_from_slice(chunk);
            let n = u64::from_be_bytes([0, 0, 0, buf[0], buf[1], buf[2], buf[3], buf[4]]);
            for i in 0..(chunk.len() * 8).div_ceil(5) {
                out.push(ALPHABET[((n >> (35 - i * 5)) & 31) as usize] as char);
            }
        }
        out
    }

    /// Test vectors from RFC 6238, appendix B.
    #[test]
    fn matches_rfc_6238_vectors() {
        type Vectors = [(u64, &'static str); 3];
        let cases: [(&str, &[u8], Vectors); 3] = [
            (
                "SHA1",
                b"12345678901234567890",
                [
                    (59, "94287082"),
                    (1111111109, "07081804"),
                    (2000000000, "69279037"),
                ],
            ),
            (
                "SHA256",
                b"12345678901234567890123456789012",
                [
                    (59, "46119246"),
                    (1111111109, "68084774"),
                    (2000000000, "90698825"),
                ],
            ),
            (
                "SHA512",
                b"1234567890123456789012345678901234567890123456789012345678901234",
                [
                    (59, "90693936"),
                    (1111111109, "25091201"),
                    (2000000000, "38618901"),
                ],
            ),
        ];
        for (algorithm, secret, vectors) in cases {
            let uri = format!(
                "otpauth://totp/Test:me?secret={}&digits=8&algorithm={algorithm}",
                base32(secret)
            );
            let totp = Totp::parse(&uri).unwrap();
            for (time, expected) in vectors {
                assert_eq!(totp.code_at(time).code, expected, "{algorithm} at {time}");
            }
        }
    }

    #[test]
    fn bare_secret_uses_authenticator_defaults() {
        // the same secret as typed from a site: lower case, in groups of four
        let totp = Totp::parse("gezd gnbv gy3t qojq gezd gnbv gy3t qojq").unwrap();
        let code = totp.code_at(59);
        assert_eq!(code.code, "287082");
        assert_eq!(code.period, 30);
        assert_eq!(code.remaining, 1);
        assert_eq!(totp.code_at(60).remaining, 30);
    }

    #[test]
    fn rejects_garbage() {
        for bad in [
            "",
            "not base32!",
            "ABC",
            "otpauth://hotp/x?secret=GEZDGNBVGY3TQOJQ",
            "otpauth://totp/x?digits=6",
            "otpauth://totp/x?secret=GEZDGNBVGY3TQOJQ&digits=12",
            "otpauth://totp/x?secret=GEZDGNBVGY3TQOJQ&algorithm=MD5",
        ] {
            assert!(Totp::parse(bad).is_err(), "{bad}");
        }
    }
}
