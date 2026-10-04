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
        let allowed = Self::is_allowed(name);
        allowed.then_some(name).ok_or_else(Self::refused)
    }

    fn is_allowed(name: &str) -> bool {
        let bytes = name.as_bytes();
        let checks = [
            (Self::MIN_LENGTH..=Self::MAX_LENGTH).contains(&bytes.len()),
            bytes.first().is_some_and(u8::is_ascii_alphanumeric),
            bytes.last().is_some_and(u8::is_ascii_alphanumeric),
            bytes.iter().copied().all(Self::is_allowed_byte),
            !Self::RESERVED.contains(&name),
        ];
        checks == [true; 5]
    }

    fn is_allowed_byte(byte: u8) -> bool {
        let kinds = [
            byte.is_ascii_lowercase(),
            byte.is_ascii_digit(),
            byte == b'-',
        ];
        kinds.contains(&true)
    }

    fn refused() -> Failure {
        failure(
            "FYLO collection names have 2 to 64 lowercase letters, digits and hyphens, start and end with a letter or digit, and are not reserved words.",
        )
    }
}
