//! Cryptographic building blocks. Only audited RustCrypto implementations are used:
//! Argon2id for key derivation, XChaCha20-Poly1305 for authenticated encryption and
//! the OS CSPRNG (`getrandom`) for every random value.

pub mod aead;
pub mod generator;
pub mod kdf;
pub mod random;
pub mod totp;

/// Version of the item/envelope encryption format stored in every row.
/// Bump it (and keep the old decoder) if the scheme ever changes.
pub const CRYPTO_VERSION: i64 = 1;
