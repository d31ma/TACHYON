use serde_json::Value;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StreamEventName {
    name: String,
}

impl StreamEventName {
    pub(crate) const PREFIX: &'static str = "/.tachyon/event/";
    const MOST_LENGTH: usize = 64;
    const NAME_MARKS: &'static [u8; 3] = b".-_";
    const RESERVED: [&'static str; 4] = ["message", "end", "error", "heartbeat"];
    const FORM: &'static str = "event names the server-sent event type that carries the messages, the text after `event:` in the stream, such as \"state\". A name starts with a letter and has at most 64 letters, digits, dots, dashes and underscores. Leave event out for messages without a name";
    const TAKEN: &'static str = "event cannot be \"message\", \"end\", \"error\" or \"heartbeat\": Tachyon reads those itself. Leave event out for messages without a name";

    pub(crate) fn read(value: &Value) -> Result<Self, String> {
        let name = value
            .as_str()
            .filter(|name| Self::is_name(name))
            .ok_or_else(|| String::from(Self::FORM))?;
        Self::refuse_reserved(name)?;
        Ok(Self {
            name: name.to_owned(),
        })
    }

    pub(crate) fn addressed(&self, path: &str) -> String {
        format!("{}{}{path}", Self::PREFIX, self.name)
    }

    fn is_name(name: &str) -> bool {
        let starts_with_letter = name.starts_with(|letter: char| letter.is_ascii_alphabetic());
        let short = name.len() <= Self::MOST_LENGTH;
        let plain = name.bytes().all(Self::is_name_byte);
        [starts_with_letter, short, plain] == [true, true, true]
    }

    fn is_name_byte(byte: u8) -> bool {
        [
            byte.is_ascii_alphanumeric(),
            Self::NAME_MARKS.contains(&byte),
        ]
        .contains(&true)
    }

    fn refuse_reserved(name: &str) -> Result<(), String> {
        let free = !Self::RESERVED.contains(&name);
        free.then_some(()).ok_or_else(|| String::from(Self::TAKEN))
    }
}
