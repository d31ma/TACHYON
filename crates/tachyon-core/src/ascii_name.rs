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

    fn is_name_byte(byte: u8) -> bool {
        byte.is_ascii_alphanumeric() || byte == b'_'
    }
}
