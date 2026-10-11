use super::MethodHeader;

pub(super) struct RubyEmptyMethod;

impl RubyEmptyMethod {
    const DEFINITION: &'static str = "def ";
    const END: &'static str = "end";
    const PARAMETERS: char = '(';
    const SKIPPED: [char; 6] = [')', ' ', '\t', '\r', '\n', ';'];

    // The body is read from the source: masking blanks string literals, so a method
    // that only returns "text" would otherwise look empty.
    pub(super) fn is_empty(source: &str, masked: &str, line: usize) -> bool {
        let start: usize = masked.split_inclusive('\n').take(line).map(str::len).sum();
        let text = masked.get(start..).unwrap_or_default();
        let rest = source
            .get(start + Self::signature_end(text)..)
            .unwrap_or_default();
        Self::starts_with_end(rest.trim_start_matches(Self::SKIPPED))
    }

    fn signature_end(text: &str) -> usize {
        let name_end = Self::name_end(text);
        let opens = text[name_end..].starts_with(Self::PARAMETERS);
        let closed = MethodHeader::closing(text, name_end).unwrap_or(name_end);
        [name_end, closed][usize::from(opens)]
    }

    fn name_end(text: &str) -> usize {
        let name_start = text
            .find(Self::DEFINITION)
            .map_or(0, |at| at + Self::DEFINITION.len());
        let length = text[name_start..]
            .find(|character: char| !Self::is_name_character(character))
            .unwrap_or(text.len() - name_start);
        name_start + length
    }

    fn starts_with_end(body: &str) -> bool {
        body.strip_prefix(Self::END)
            .is_some_and(|rest| !rest.starts_with(Self::is_name_character))
    }

    fn is_name_character(character: char) -> bool {
        [character.is_ascii_alphanumeric(), character == '_'].contains(&true)
    }
}
