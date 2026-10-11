pub(crate) struct AsciiName;

impl AsciiName {
    /// Reads the ASCII letters, digits and underscores that start `text`.
    pub(crate) fn leading(text: &str) -> &str {
        let length = text
            .bytes()
            .take_while(|byte| Self::is_name_byte(*byte))
            .count();
        &text[..length]
    }

    pub(crate) fn is_name_byte(byte: u8) -> bool {
        [byte.is_ascii_alphanumeric(), byte == b'_'].contains(&true)
    }
}

#[cfg(test)]
mod tests {
    use super::AsciiName;

    #[test]
    fn name_bytes_are_the_ascii_letters_digits_and_underscore() {
        for byte in 0..=u8::MAX {
            let expected = byte.is_ascii_alphanumeric() || byte == b'_';
            assert_eq!(AsciiName::is_name_byte(byte), expected, "{byte}");
        }
    }
}
