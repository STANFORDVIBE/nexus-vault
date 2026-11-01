//! Password and passphrase generator backed by the OS CSPRNG.

use rand::RngExt;
use rand::rand_core::UnwrapErr;
use rand::rngs::SysRng;
use serde::{Deserialize, Serialize};

use crate::core::error::{AppError, AppResult};

const UPPER: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const LOWER: &str = "abcdefghijklmnopqrstuvwxyz";
const DIGITS: &str = "0123456789";
const SYMBOLS: &str = "!@#$%^&*()-_=+[]{};:,.<>?/~";
/// Characters that are easy to confuse when read or typed by hand.
const AMBIGUOUS: &str = "Il1O0o|`'\";:,.";

pub const MIN_LENGTH: usize = 8;
pub const MAX_LENGTH: usize = 128;
pub const MIN_WORDS: usize = 3;
pub const MAX_WORDS: usize = 12;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GeneratorMode {
    Password,
    Passphrase,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratorOptions {
    pub mode: GeneratorMode,
    pub length: usize,
    pub uppercase: bool,
    pub lowercase: bool,
    pub digits: bool,
    pub symbols: bool,
    pub exclude_ambiguous: bool,
    pub words: usize,
    pub separator: String,
    pub capitalize: bool,
    pub include_number: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedPassword {
    pub value: String,
    pub entropy_bits: f64,
}

pub fn generate(options: &GeneratorOptions) -> AppResult<GeneratedPassword> {
    match options.mode {
        GeneratorMode::Password => generate_password(options),
        GeneratorMode::Passphrase => generate_passphrase(options),
    }
}

fn rng() -> UnwrapErr<SysRng> {
    UnwrapErr(SysRng)
}

fn generate_password(options: &GeneratorOptions) -> AppResult<GeneratedPassword> {
    if !(MIN_LENGTH..=MAX_LENGTH).contains(&options.length) {
        return Err(AppError::Validation("length"));
    }

    let classes: Vec<Vec<char>> = [
        (options.uppercase, UPPER),
        (options.lowercase, LOWER),
        (options.digits, DIGITS),
        (options.symbols, SYMBOLS),
    ]
    .into_iter()
    .filter(|(enabled, _)| *enabled)
    .map(|(_, set)| {
        set.chars()
            .filter(|c| !options.exclude_ambiguous || !AMBIGUOUS.contains(*c))
            .collect()
    })
    .collect();

    if classes.is_empty() {
        return Err(AppError::Validation("charset"));
    }
    let alphabet: Vec<char> = classes.iter().flatten().copied().collect();
    let mut rng = rng();

    // Rejection sampling keeps every character uniform while guaranteeing that each
    // selected class appears at least once (expected retries are tiny for length >= 8).
    let value = loop {
        let candidate: String = (0..options.length)
            .map(|_| alphabet[rng.random_range(0..alphabet.len())])
            .collect();
        if classes
            .iter()
            .all(|class| candidate.chars().any(|c| class.contains(&c)))
        {
            break candidate;
        }
    };

    let entropy_bits = options.length as f64 * (alphabet.len() as f64).log2();
    Ok(GeneratedPassword {
        value,
        entropy_bits,
    })
}

fn generate_passphrase(options: &GeneratorOptions) -> AppResult<GeneratedPassword> {
    if !(MIN_WORDS..=MAX_WORDS).contains(&options.words) {
        return Err(AppError::Validation("words"));
    }
    if options.separator.chars().count() > 3 {
        return Err(AppError::Validation("separator"));
    }

    let list = bip39::Language::English.word_list();
    let mut rng = rng();
    let mut words: Vec<String> = (0..options.words)
        .map(|_| {
            let word = list[rng.random_range(0..list.len())];
            if options.capitalize {
                let mut chars = word.chars();
                chars.next().map_or_else(String::new, |first| {
                    first.to_uppercase().chain(chars).collect()
                })
            } else {
                word.to_string()
            }
        })
        .collect();

    let mut entropy_bits = options.words as f64 * (list.len() as f64).log2();
    if options.include_number {
        let index = rng.random_range(0..words.len());
        let digit = rng.random_range(0..10u8);
        words[index].push(char::from(b'0' + digit));
        entropy_bits += 10f64.log2() + (words.len() as f64).log2();
    }

    Ok(GeneratedPassword {
        value: words.join(&options.separator),
        entropy_bits,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn password_options() -> GeneratorOptions {
        GeneratorOptions {
            mode: GeneratorMode::Password,
            length: 20,
            uppercase: true,
            lowercase: true,
            digits: true,
            symbols: true,
            exclude_ambiguous: false,
            words: 5,
            separator: "-".into(),
            capitalize: false,
            include_number: false,
        }
    }

    #[test]
    fn password_respects_length_and_classes() {
        for _ in 0..200 {
            let out = generate(&password_options()).unwrap();
            assert_eq!(out.value.chars().count(), 20);
            assert!(out.value.chars().any(|c| c.is_ascii_uppercase()));
            assert!(out.value.chars().any(|c| c.is_ascii_lowercase()));
            assert!(out.value.chars().any(|c| c.is_ascii_digit()));
            assert!(out.value.chars().any(|c| SYMBOLS.contains(c)));
        }
    }

    #[test]
    fn excludes_ambiguous_characters() {
        let options = GeneratorOptions {
            exclude_ambiguous: true,
            length: 128,
            ..password_options()
        };
        for _ in 0..50 {
            let out = generate(&options).unwrap();
            assert!(!out.value.chars().any(|c| AMBIGUOUS.contains(c)));
        }
    }

    #[test]
    fn digits_only_has_expected_entropy() {
        let options = GeneratorOptions {
            uppercase: false,
            lowercase: false,
            symbols: false,
            length: 10,
            ..password_options()
        };
        let out = generate(&options).unwrap();
        assert!(out.value.chars().all(|c| c.is_ascii_digit()));
        assert!((out.entropy_bits - 10.0 * 10f64.log2()).abs() < 1e-9);
    }

    #[test]
    fn rejects_invalid_options() {
        let short = GeneratorOptions {
            length: 7,
            ..password_options()
        };
        assert!(generate(&short).is_err());
        let long = GeneratorOptions {
            length: 129,
            ..password_options()
        };
        assert!(generate(&long).is_err());
        let empty = GeneratorOptions {
            uppercase: false,
            lowercase: false,
            digits: false,
            symbols: false,
            ..password_options()
        };
        assert!(generate(&empty).is_err());
    }

    #[test]
    fn passphrase_has_requested_words() {
        let options = GeneratorOptions {
            mode: GeneratorMode::Passphrase,
            words: 6,
            capitalize: true,
            include_number: true,
            ..password_options()
        };
        let out = generate(&options).unwrap();
        let parts: Vec<&str> = out.value.split('-').collect();
        assert_eq!(parts.len(), 6);
        assert!(
            parts
                .iter()
                .all(|w| w.chars().next().unwrap().is_uppercase())
        );
        assert_eq!(out.value.chars().filter(|c| c.is_ascii_digit()).count(), 1);
        assert!(out.entropy_bits > 66.0);
    }

    #[test]
    fn outputs_differ() {
        let a = generate(&password_options()).unwrap().value;
        let b = generate(&password_options()).unwrap().value;
        assert_ne!(a, b);
    }
}
