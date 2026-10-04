use crate::Failure;
use crate::storage::failure;

pub(in crate::storage) struct CollectionName;

impl CollectionName {
    const MIN_LENGTH: usize = 2;
    const MAX_LENGTH: usize = 64;
    const RESERVED: [&'static str; 11] = [
        "sql", "as", "then", "db", "engine", "cache", "queue", "startup", "join", "ready", "close",
    ];

    pub(in crate::storage) fn checked(name: &str) -> Result<&str, Failure> {
        if !Self::is_allowed(name) {
            return Err(failure(
                "FYLO collection names have 2 to 64 lowercase letters, digits and hyphens, start and end with a letter or digit, and are not reserved words.",
            ));
        }
        Ok(name)
    }

    fn is_allowed(name: &str) -> bool {
        let bytes = name.as_bytes();
        let has_allowed_length = bytes.len() >= Self::MIN_LENGTH && bytes.len() <= Self::MAX_LENGTH;
        let starts_well = bytes.first().is_some_and(u8::is_ascii_alphanumeric);
        let ends_well = bytes.last().is_some_and(u8::is_ascii_alphanumeric);
        let has_allowed_bytes = bytes.iter().all(|byte| Self::is_allowed_byte(*byte));
        let is_reserved = Self::RESERVED.contains(&name);
        has_allowed_length && starts_well && ends_well && has_allowed_bytes && !is_reserved
    }

    fn is_allowed_byte(byte: u8) -> bool {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'
    }
}
