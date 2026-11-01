use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

use crate::core::error::{AppError, AppResult};
use crate::crypto::random;

pub const KEY_LEN: usize = 32;
pub const NONCE_LEN: usize = 24;

/// 256-bit symmetric key, wiped from memory when dropped. Deliberately not `Clone`,
/// `Debug` or `Serialize`, so it cannot be copied around or logged by accident.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct SecretKey([u8; KEY_LEN]);

impl SecretKey {
    pub fn from_bytes(bytes: [u8; KEY_LEN]) -> Self {
        SecretKey(bytes)
    }

    pub fn generate() -> AppResult<Self> {
        Ok(SecretKey(random::bytes()?))
    }

    pub fn expose(&self) -> &[u8; KEY_LEN] {
        &self.0
    }

    fn cipher(&self) -> XChaCha20Poly1305 {
        XChaCha20Poly1305::new(&Key::from(self.0))
    }
}

/// Ciphertext together with the nonce it was produced with.
pub struct Sealed {
    pub nonce: [u8; NONCE_LEN],
    pub ciphertext: Vec<u8>,
}

/// Encrypts `plaintext` under `key`, binding it to `aad` (which is authenticated
/// but not encrypted). A fresh random 192-bit nonce is drawn for every call, so
/// nonce reuse is not a practical concern for XChaCha20.
pub fn seal(key: &SecretKey, plaintext: &[u8], aad: &[u8]) -> AppResult<Sealed> {
    let nonce: [u8; NONCE_LEN] = random::bytes()?;
    let ciphertext = key
        .cipher()
        .encrypt(
            &XNonce::from(nonce),
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| AppError::Internal)?;
    Ok(Sealed { nonce, ciphertext })
}

/// Like [`seal`], for formats where the nonce must be known before encrypting (it is
/// part of authenticated header data). The caller must pass a freshly drawn random nonce.
pub fn seal_with_nonce(
    key: &SecretKey,
    nonce: &[u8; NONCE_LEN],
    plaintext: &[u8],
    aad: &[u8],
) -> AppResult<Vec<u8>> {
    key.cipher()
        .encrypt(
            &XNonce::from(*nonce),
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| AppError::Internal)
}

/// Decrypts and authenticates. Any change to the ciphertext, nonce, AAD or key yields
/// `AppError::Integrity`; the plaintext is wiped from memory when the result is dropped.
pub fn open(
    key: &SecretKey,
    nonce: &[u8],
    ciphertext: &[u8],
    aad: &[u8],
) -> AppResult<Zeroizing<Vec<u8>>> {
    let nonce: [u8; NONCE_LEN] = nonce.try_into().map_err(|_| AppError::Integrity)?;
    key.cipher()
        .decrypt(
            &XNonce::from(nonce),
            Payload {
                msg: ciphertext,
                aad,
            },
        )
        .map(Zeroizing::new)
        .map_err(|_| AppError::Integrity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn roundtrip() {
        let key = SecretKey::generate().unwrap();
        let sealed = seal(&key, b"hunter2", b"aad").unwrap();
        assert_ne!(sealed.ciphertext.as_slice(), b"hunter2");
        let plain = open(&key, &sealed.nonce, &sealed.ciphertext, b"aad").unwrap();
        assert_eq!(plain.as_slice(), b"hunter2");
    }

    #[test]
    fn tampered_ciphertext_fails_authentication() {
        let key = SecretKey::generate().unwrap();
        let mut sealed = seal(&key, b"hunter2", b"aad").unwrap();
        sealed.ciphertext[0] ^= 0x01;
        assert!(matches!(
            open(&key, &sealed.nonce, &sealed.ciphertext, b"aad"),
            Err(AppError::Integrity)
        ));
    }

    #[test]
    fn wrong_key_nonce_or_aad_fails() {
        let key = SecretKey::generate().unwrap();
        let other = SecretKey::generate().unwrap();
        let sealed = seal(&key, b"hunter2", b"aad").unwrap();

        assert!(open(&other, &sealed.nonce, &sealed.ciphertext, b"aad").is_err());
        assert!(open(&key, &sealed.nonce, &sealed.ciphertext, b"other-aad").is_err());
        let mut nonce = sealed.nonce;
        nonce[0] ^= 1;
        assert!(open(&key, &nonce, &sealed.ciphertext, b"aad").is_err());
        assert!(open(&key, &sealed.nonce[..12], &sealed.ciphertext, b"aad").is_err());
    }

    #[test]
    fn nonces_never_repeat() {
        let key = SecretKey::generate().unwrap();
        let nonces: HashSet<[u8; NONCE_LEN]> = (0..10_000)
            .map(|_| seal(&key, b"x", b"").unwrap().nonce)
            .collect();
        assert_eq!(nonces.len(), 10_000);
    }

    #[test]
    fn same_plaintext_encrypts_differently() {
        let key = SecretKey::generate().unwrap();
        let a = seal(&key, b"same", b"").unwrap();
        let b = seal(&key, b"same", b"").unwrap();
        assert_ne!(a.ciphertext, b.ciphertext);
    }
}
