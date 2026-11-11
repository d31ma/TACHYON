use super::words::Words;

pub(super) struct KotlinElvisReturn<'r, 'a> {
    words: &'r Words<'a>,
}

impl<'r, 'a> KotlinElvisReturn<'r, 'a> {
    const ELVIS: &'static str = "?:";
    const RETURN: &'static str = "return";
    const DECLARATIONS: &'static [&'static str] = &["val", "var"];
    const RETURN_OFFSET: usize = 2;

    pub(super) fn new(words: &'r Words<'a>) -> Self {
        Self { words }
    }

    pub(super) fn matches(&self, at: usize) -> bool {
        let depth = self.words.depth(at);
        let returned = at + Self::RETURN_OFFSET;
        let start = self.words.statement_start(at, depth);
        (self.words.pair(at) == Self::ELVIS)
            & (self.words.text(returned) == Self::RETURN)
            & Self::DECLARATIONS.contains(&self.words.text(start))
            & self.ends_after_value(returned + 1, depth)
    }

    fn ends_after_value(&self, at: usize, depth: usize) -> bool {
        let bare = self.words.ends_statement(at, depth);
        let valued = self.words.is_simple_value(at) & self.words.ends_statement(at + 1, depth);
        bare | valued
    }
}
