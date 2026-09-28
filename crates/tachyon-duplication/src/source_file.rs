use crate::code_token::CodeToken;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceFile {
    pub path: String,
    pub language: String,
    pub tokens: Vec<CodeToken>,
}

impl SourceFile {
    #[must_use]
    pub fn new(
        path: impl Into<String>,
        language: impl Into<String>,
        tokens: Vec<CodeToken>,
    ) -> Self {
        Self {
            path: path.into(),
            language: language.into(),
            tokens,
        }
    }
}
