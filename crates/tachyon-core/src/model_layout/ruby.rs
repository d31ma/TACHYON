use super::piece::Piece;

#[derive(Default)]
struct RubyModelScan {
    in_class: bool,
    constructor_indent: Option<usize>,
    open_groups: usize,
}

impl RubyModelScan {
    const RECORD_BUILDERS: [&'static str; 2] = ["Data.define(", "Struct.new("];
    const ATTRIBUTES: [&'static str; 2] = ["attr_reader ", "attr_accessor "];

    fn read(&mut self, line: &str) -> bool {
        let text = line.trim();
        if text.is_empty() {
            return true;
        }
        if self.open_groups > 0 {
            self.open_groups = Self::groups_after(text, self.open_groups);
            return true;
        }
        let indent = line.len() - line.trim_start().len();
        if indent == 0 {
            return self.read_top_level(text);
        }
        self.read_member(text, indent)
    }

    fn read_top_level(&mut self, text: &str) -> bool {
        if text == "end" && self.in_class {
            self.in_class = false;
            return true;
        }
        if Self::is_require(text) {
            return !self.in_class;
        }
        if let Some(header) = text.strip_prefix("class ") {
            return self.read_class(header);
        }
        self.read_record_constant(text)
    }

    fn read_class(&mut self, header: &str) -> bool {
        if self.in_class {
            return false;
        }
        let name = Self::constant(header);
        let rest = header[name.len()..].trim();
        let parent = rest.strip_prefix('<').map(str::trim);
        let allowed = match parent {
            None => rest.is_empty(),
            Some(parent) => Self::is_record_builder(parent) || Self::constant(parent) == parent,
        };
        if name.is_empty() || !allowed {
            return false;
        }
        self.in_class = true;
        self.open_groups = Self::groups_after(header, 0);
        true
    }

    fn read_record_constant(&mut self, text: &str) -> bool {
        let name = Self::constant(text);
        let Some(value) = text[name.len()..].trim_start().strip_prefix('=') else {
            return false;
        };
        let value = value.trim();
        if name.is_empty() || !Self::is_record_builder(value) || value.ends_with(" do") {
            return false;
        }
        self.open_groups = Self::groups_after(value, 0);
        true
    }

    fn read_member(&mut self, text: &str, indent: usize) -> bool {
        if !self.in_class {
            return false;
        }
        if let Some(constructor) = self.constructor_indent {
            if indent > constructor {
                return true;
            }
            if text == "end" {
                self.constructor_indent = None;
                return true;
            }
        }
        if Self::ATTRIBUTES.iter().any(|name| text.starts_with(name)) {
            return true;
        }
        if text == "def initialize" || text.starts_with("def initialize(") {
            self.constructor_indent = Some(indent);
            self.open_groups = Self::groups_after(text, 0);
            return true;
        }
        false
    }

    fn is_require(text: &str) -> bool {
        text.starts_with("require ") || text.starts_with("require_relative ")
    }

    fn is_record_builder(text: &str) -> bool {
        Self::RECORD_BUILDERS
            .iter()
            .any(|builder| text.starts_with(builder))
    }

    fn constant(text: &str) -> &str {
        let length = text
            .bytes()
            .take_while(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b':'))
            .count();
        let name = &text[..length];
        if name.as_bytes().first().is_some_and(u8::is_ascii_uppercase) {
            return name;
        }
        ""
    }

    fn groups_after(text: &str, mut open: usize) -> usize {
        for byte in text.bytes() {
            match byte {
                b'(' | b'[' | b'{' => open += 1,
                b')' | b']' | b'}' => open = open.saturating_sub(1),
                _ => {}
            }
        }
        open
    }
}

pub(super) fn first_refused(code: Piece<'_>) -> Option<usize> {
    let mut scan = RubyModelScan::default();
    let mut offset = 0;
    for line in code.text.split_inclusive('\n') {
        let at = offset + line.len() - line.trim_start().len();
        offset += line.len();
        if !scan.read(line) {
            return Some(code.at + at);
        }
    }
    if scan.in_class {
        return Some(code.at + code.text.len());
    }
    None
}
