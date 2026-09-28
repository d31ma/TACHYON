#[Service]
#[NeedsAll]
struct PagelinksService {
    source: String,
    path: String,
}

impl PagelinksService {
    const REPOSITORY: &'static str = "https://github.com/d31ma/Tachyon";

    fn edit_href(&self) -> String {
        format!("{}/edit/main/{}", Self::REPOSITORY, Self::source_or_index(&self.source))
    }

    fn report_href(&self) -> String {
        let title = Self::encode_component(&format!("Docs: {}", self.path));
        format!("{}/issues/new?title={title}", Self::REPOSITORY)
    }

    fn source_or_index(source: &str) -> &str {
        if source.is_empty() {
            return "website/content/docs/page.json";
        }
        source
    }

    fn encode_component(value: &str) -> String {
        value.bytes().map(Self::encode_byte).collect()
    }

    fn encode_byte(byte: u8) -> String {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')' => char::from(byte).to_string(),
            _ => format!("%{byte:02X}"),
        }
    }
}
