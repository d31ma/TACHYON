use super::dom_bindings::DomBindings;
use super::dom_kind::DomKind;
use crate::annotation_layout::single_class::Token;

pub(super) struct DomObject<'a, 's> {
    tokens: &'a [Token<'s>],
    bindings: &'a DomBindings,
}

impl<'a, 's> DomObject<'a, 's> {
    const PUNCTUATION: [&'static str; 9] = ["{", "}", "[", "]", ",", ":", "-", "+", "."];
    const CONSTANTS: [&'static str; 4] = ["true", "false", "null", "undefined"];

    pub(super) fn new(tokens: &'a [Token<'s>], bindings: &'a DomBindings) -> Self {
        Self { tokens, bindings }
    }

    pub(super) fn kind(&self, start: usize, end: usize) -> DomKind {
        let data = (start..end).all(|at| self.holds_data(at));
        data.then_some(DomKind::Data).unwrap_or_default()
    }

    fn holds_data(&self, at: usize) -> bool {
        let text = self.text(at);
        [
            self.tokens.get(at).is_some_and(|token| token.literal),
            Self::PUNCTUATION.contains(&text),
            Self::CONSTANTS.contains(&text),
            text.as_bytes().first().is_some_and(u8::is_ascii_digit),
            self.text(at + 1) == ":",
            self.bindings.kind(text) == DomKind::Data,
        ]
        .contains(&true)
    }

    fn text(&self, at: usize) -> &'s str {
        self.tokens.get(at).map_or("", |token| token.text)
    }
}
