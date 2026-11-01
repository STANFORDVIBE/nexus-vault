use argon2::{Algorithm, Argon2, Params, Version};
use zeroize::Zeroizing;

use crate::core::error::{AppError, AppResult};
use crate::crypto::aead::SecretKey;

pub const KDF_ALGORITHM: &str = "argon2id-v19";
pub const SALT_LEN: usize = 16;

/// Argon2id cost parameters, stored next to the salt so they can be raised later
/// without breaking existing vaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KdfParams {
    pub memory_kib: u32,
    pub iterations: u32,
    pub parallelism: u32,
}

impl KdfParams {
    /// Chosen by benchmarking on the target machine (see docs/SECURITY.md):
    /// 256 MiB, 3 passes, 4 lanes. Well above the OWASP minimum (19 MiB / 2 passes).
    pub const DEFAULT: KdfParams = KdfParams {
        memory_kib: 256 * 1024,
        iterations: 3,
        parallelism: 4,
    };

    /// Floor below which a stored vault is considered tampered with or corrupt.
    const MIN: KdfParams = KdfParams {
        memory_kib: 19 * 1024,
        iterations: 2,
        parallelism: 1,
    };

    /// Deliberately cheap parameters so the test-suite stays fast. Never used in builds.
    #[cfg(test)]
    pub const TEST: KdfParams = KdfParams {
        memory_kib: 8,
        iterations: 1,
        parallelism: 1,
    };

    /// Rejects parameters an attacker could have downgraded in the database file.
    pub fn validate_stored(&self) -> AppResult<()> {
        #[cfg(test)]
        if *self == Self::TEST {
            return Ok(());
        }
        if self.memory_kib < Self::MIN.memory_kib
            || self.iterations < Self::MIN.iterations
            || self.parallelism < Self::MIN.parallelism
            || self.memory_kib > 4 * 1024 * 1024
            || self.iterations > 64
            || self.parallelism > 64
        {
            return Err(AppError::Integrity);
        }
        Ok(())
    }
}

/// Derives the 256-bit Key Encryption Key from the master password.
pub fn derive_kek(password: &[u8], salt: &[u8], params: KdfParams) -> AppResult<SecretKey> {
    let argon_params = Params::new(
        params.memory_kib,
        params.iterations,
        params.parallelism,
        Some(32),
    )
    .map_err(|_| AppError::Integrity)?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon_params);

    let mut out = Zeroizing::new([0u8; 32]);
    argon
        .hash_password_into(password, salt, out.as_mut())
        .map_err(|_| AppError::Internal)?;
    Ok(SecretKey::from_bytes(*out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_input_same_key_different_salt_different_key() {
        let a = derive_kek(b"correct horse", &[1; 16], KdfParams::TEST).unwrap();
        let b = derive_kek(b"correct horse", &[1; 16], KdfParams::TEST).unwrap();
        let c = derive_kek(b"correct horse", &[2; 16], KdfParams::TEST).unwrap();
        let d = derive_kek(b"correct horsf", &[1; 16], KdfParams::TEST).unwrap();
        assert_eq!(a.expose(), b.expose());
        assert_ne!(a.expose(), c.expose());
        assert_ne!(a.expose(), d.expose());
    }

    #[test]
    fn rejects_downgraded_parameters() {
        assert!(KdfParams::DEFAULT.validate_stored().is_ok());
        let weak = KdfParams {
            memory_kib: 1024,
            iterations: 1,
            parallelism: 1,
        };
        assert!(matches!(weak.validate_stored(), Err(AppError::Integrity)));
    }

    /// Prints how long a production-strength derivation takes on this machine.
    /// Run with: cargo test --release -- --ignored kdf_benchmark --nocapture
    #[test]
    #[ignore]
    #[allow(clippy::print_stdout)]
    fn kdf_benchmark() {
        for params in [
            KdfParams {
                memory_kib: 64 * 1024,
                iterations: 3,
                parallelism: 4,
            },
            KdfParams {
                memory_kib: 128 * 1024,
                iterations: 3,
                parallelism: 4,
            },
            KdfParams::DEFAULT,
            KdfParams {
                memory_kib: 512 * 1024,
                iterations: 3,
                parallelism: 4,
            },
        ] {
            let start = std::time::Instant::now();
            derive_kek(b"benchmark password", &[7; 16], params).unwrap();
            println!("{params:?}: {:?}", start.elapsed());
        }
    }
}
