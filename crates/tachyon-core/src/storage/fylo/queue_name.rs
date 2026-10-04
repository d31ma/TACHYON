use crate::Failure;
use crate::storage::failure;

pub(super) struct QueueName;

impl QueueName {
    const MAX_LENGTH: usize = 127;

    pub(super) fn checked(name: &str) -> Result<&str, Failure> {
        if !Self::is_allowed(name) {
            return Err(failure(
                "FYLO queue topics and groups have 1 to 127 ASCII letters, digits, dots, hyphens and underscores, and start with a letter or digit.",
            ));
        }
        Ok(name)
    }

    fn is_allowed(name: &str) -> bool {
        let bytes = name.as_bytes();
        let has_allowed_length = !bytes.is_empty() && bytes.len() <= Self::MAX_LENGTH;
        let starts_well = bytes.first().is_some_and(u8::is_ascii_alphanumeric);
        let has_allowed_bytes = bytes.iter().all(|byte| Self::is_allowed_byte(*byte));
        has_allowed_length && starts_well && has_allowed_bytes
    }

    fn is_allowed_byte(byte: u8) -> bool {
        byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_')
    }
}
