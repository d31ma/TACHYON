use crate::Failure;
use crate::storage::failure;

pub(super) struct QueueName;

impl QueueName {
    const MAX_LENGTH: usize = 127;
    const PUNCTUATION: [u8; 3] = *b".-_";

    pub(super) fn checked(name: &str) -> Result<&str, Failure> {
        let allowed = Self::is_allowed(name);
        allowed.then_some(name).ok_or_else(Self::refused)
    }

    fn is_allowed(name: &str) -> bool {
        let bytes = name.as_bytes();
        let checks = [
            (1..=Self::MAX_LENGTH).contains(&bytes.len()),
            bytes.first().is_some_and(u8::is_ascii_alphanumeric),
            bytes.iter().copied().all(Self::is_allowed_byte),
        ];
        checks == [true; 3]
    }

    fn is_allowed_byte(byte: u8) -> bool {
        let kinds = [
            byte.is_ascii_alphanumeric(),
            Self::PUNCTUATION.contains(&byte),
        ];
        kinds.contains(&true)
    }

    fn refused() -> Failure {
        failure(
            "FYLO queue topics and groups have 1 to 127 ASCII letters, digits, dots, hyphens and underscores, and start with a letter or digit.",
        )
    }
}
