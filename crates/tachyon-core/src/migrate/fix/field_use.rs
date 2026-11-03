use super::scan::SourceText;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum FieldUse {
    Rename,
    Keep,
    Unsure,
}

impl FieldUse {
    const SELF_WORDS: [&'static str; 3] = ["this", "self", "$this"];
    const IMPLICIT_SELF: [&'static str; 6] = ["java", "kt", "cs", "swift", "dart", "rb"];
    const MEMBER_BEFORE: [&'static str; 4] = ["?.", "!!.", "->", "."];
    const MEMBER_AFTER: [&'static str; 4] = ["?.", "!!.", "!.", "."];
    const TYPE_BEFORE: [&'static str; 4] = [":", "<", "new", "is"];
    const TYPE_AFTER: [&'static str; 4] = ["<", "(", "?", "!"];
    const GO_RECEIVER: &'static str = "func (";
    const RUBY_VARIABLE: u8 = b'@';

    pub(super) fn of(text: &SourceText<'_>, at: usize, name: &str, language: &str) -> Self {
        let receiver = Self::receiver_before(text, at);
        let owned = receiver.is_some_and(|word| Self::receivers(text, language).contains(&word));
        let own_member = [owned, Self::ruby_variable(text, at, language)].contains(&true);
        let other_member = receiver.is_some();
        let explicit_self = !Self::IMPLICIT_SELF.contains(&language);
        let lowercase = !name.starts_with(|letter: char| letter.is_ascii_uppercase());
        let after = &text.source[at + name.len()..];
        let rules = [
            (own_member, Self::Rename),
            (other_member, Self::Keep),
            (explicit_self, Self::Keep),
            (lowercase, Self::Rename),
            (language == "rb", Self::Keep),
            (Self::member_after(after), Self::Rename),
            (Self::is_type_use(text, at, after), Self::Keep),
        ];
        rules
            .into_iter()
            .find(|(applies, _)| *applies)
            .map_or(Self::Unsure, |(_, verdict)| verdict)
    }

    fn member_after(after: &str) -> bool {
        Self::MEMBER_AFTER
            .iter()
            .any(|mark| after.starts_with(mark))
    }

    fn is_type_use(text: &SourceText<'_>, at: usize, after: &str) -> bool {
        let before = Self::code_before(text, at);
        let next = after.trim_start();
        let named_after = next.starts_with(|letter: char| letter.is_alphanumeric());
        let marked_after = Self::TYPE_AFTER.iter().any(|mark| next.starts_with(mark));
        let marked_before = Self::TYPE_BEFORE.iter().any(|mark| before.ends_with(mark));
        [named_after, marked_after, marked_before].contains(&true)
    }

    fn code_before<'a>(text: &SourceText<'a>, at: usize) -> &'a str {
        let end = text.previous_code(at).map_or(0, |last| last + 1);
        &text.source[..end]
    }

    fn receiver_before<'a>(text: &SourceText<'a>, at: usize) -> Option<&'a str> {
        let before = Self::code_before(text, at);
        Self::MEMBER_BEFORE
            .iter()
            .find(|mark| before.ends_with(*mark))
            .map(|mark| text.previous_word(before.len() - mark.len()))
    }

    fn ruby_variable(text: &SourceText<'_>, at: usize, language: &str) -> bool {
        let marked = at
            .checked_sub(1)
            .is_some_and(|before| text.bytes[before] == Self::RUBY_VARIABLE);
        [language == "rb", marked] == [true, true]
    }

    fn receivers<'a>(text: &SourceText<'a>, language: &str) -> Vec<&'a str> {
        let mut receivers = Self::SELF_WORDS.to_vec();
        let go_receivers = text
            .source
            .match_indices(Self::GO_RECEIVER)
            .filter(|_| language == "go")
            .map(|(at, _)| Self::receiver_word(text.source, at + Self::GO_RECEIVER.len()));
        receivers.extend(go_receivers);
        receivers
    }

    fn receiver_word(source: &str, start: usize) -> &str {
        let rest = &source[start..];
        let end = rest
            .find(|letter: char| !Self::is_word_letter(letter))
            .unwrap_or(rest.len());
        &rest[..end]
    }

    fn is_word_letter(letter: char) -> bool {
        letter.is_alphanumeric() || letter == '_'
    }
}
