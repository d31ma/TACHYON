use crate::keywords::Keywords;
use crate::token_kind::TokenKind;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeToken {
    pub text: String,
    pub line: usize,
    pub kind: TokenKind,
}

impl CodeToken {
    const LITERAL: &'static str = "$literal";

    #[must_use]
    pub fn new(text: impl Into<String>, line: usize, kind: TokenKind) -> Self {
        Self {
            text: text.into(),
            line,
            kind,
        }
    }

    pub(crate) fn normalized(&self) -> &str {
        self.shape()
    }

    fn shape(&self) -> &str {
        match self.kind {
            TokenKind::Name => Keywords::spelling(&self.text),
            TokenKind::Literal => Self::LITERAL,
            TokenKind::Symbol => &self.text,
        }
    }
}
