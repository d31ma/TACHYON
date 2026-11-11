pub(crate) struct RemoteOrigin;

impl RemoteOrigin {
    pub(crate) const PREFIX: &'static str = "/.tachyon/origin/";
    const MOST_NAME_LENGTH: usize = 32;
    const RESERVED: [&'static str; 2] = [Self::PREFIX, crate::behavior::ReconnectPolicy::PREFIX];

    pub(crate) fn addressed(name: &str, path: &str) -> Result<String, String> {
        Self::refuse_bad_name(Self::is_name(name), name)?;
        Ok(Self::prefixed(name, path))
    }

    pub(crate) fn prefixed(name: &str, path: &str) -> String {
        format!("{}{name}{path}", Self::PREFIX)
    }

    pub(crate) fn check_written(path: &str) -> Result<(), String> {
        let reserved = Self::RESERVED.iter().any(|prefix| path.starts_with(prefix));
        Self::refuse_reserved(reserved)
    }

    pub(crate) fn is_name(name: &str) -> bool {
        let starts_with_letter = name.starts_with(|letter: char| letter.is_ascii_lowercase());
        let short = name.len() <= Self::MOST_NAME_LENGTH;
        let plain = name.bytes().all(Self::is_name_byte);
        [starts_with_letter, short, plain] == [true, true, true]
    }

    const fn is_name_byte(byte: u8) -> bool {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'
    }

    fn refuse_bad_name(good: bool, name: &str) -> Result<(), String> {
        if good {
            return Ok(());
        }
        Err(format!(
            "The client origin \"{name}\" is not a name from tac.yon.origins: a name starts with a lowercase letter and has at most 32 lowercase letters, digits and dashes, such as \"realtime\""
        ))
    }

    fn refuse_reserved(reserved: bool) -> Result<(), String> {
        if !reserved {
            return Ok(());
        }
        Err(String::from(
            "A client path cannot start with /.tachyon/origin/ or /.tachyon/reconnect/: Tachyon writes those itself. Name a server with \"origin\" instead, such as @HTTPClient({\"path\": \"/chat\", \"origin\": \"realtime\"})",
        ))
    }
}
