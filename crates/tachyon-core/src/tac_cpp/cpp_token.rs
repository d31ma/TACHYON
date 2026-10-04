#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CppToken {
    pub(crate) text: String,
    pub(crate) start: usize,
}

impl CppToken {
    pub(crate) fn end(&self) -> usize {
        self.start + self.text.len()
    }

    pub(crate) fn is(&self, text: &str) -> bool {
        self.text == text
    }

    pub(crate) fn is_word(&self) -> bool {
        self.text.starts_with(|character: char| {
            [character.is_ascii_alphabetic(), character == '_'].contains(&true)
        })
    }

    pub(crate) fn touches(&self, next: &Self) -> bool {
        self.end() == next.start
    }
}
