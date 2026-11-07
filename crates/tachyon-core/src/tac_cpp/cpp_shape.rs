use super::CppTokens;
use std::ops::Range;

pub(crate) struct CppShape {
    pub(crate) parameters: Option<usize>,
    pub(crate) cut: Option<usize>,
    pub(crate) initialized: bool,
    pub(crate) commas: Vec<usize>,
    pub(crate) operator: bool,
    angles: usize,
}

impl CppShape {
    const CUTS: [&'static str; 4] = ["=", "{", "[", ":"];
    const INITIALIZERS: [&'static str; 2] = ["=", "{"];

    pub(crate) fn of(tokens: &CppTokens, declaration: &Range<usize>) -> Self {
        let mut shape = Self {
            parameters: None,
            cut: None,
            initialized: false,
            commas: Vec::new(),
            operator: false,
            angles: 0,
        };
        shape.scan(tokens, declaration);
        shape
    }

    fn scan(&mut self, tokens: &CppTokens, declaration: &Range<usize>) {
        let mut at = declaration.start;
        while at < declaration.end {
            self.note(tokens.text(at), at);
            at = tokens.skipped(at);
        }
    }

    fn note(&mut self, text: &str, at: usize) {
        self.operator |= text == "operator";
        let open = [
            self.angles == 0,
            self.cut.is_none(),
            self.parameters.is_none(),
        ] == [true, true, true];
        let cuts_here = [open, Self::CUTS.contains(&text)] == [true, true];
        self.initialized |= [cuts_here, Self::INITIALIZERS.contains(&text)] == [true, true];
        self.cut = self.cut.or(cuts_here.then_some(at));
        self.parameters = self
            .parameters
            .or(([open, text == "("] == [true, true]).then_some(at));
        match text {
            "<" => self.angles += 1,
            ">" => self.angles = self.angles.saturating_sub(1),
            "," => self.commas.extend((self.angles == 0).then_some(at)),
            _ => {}
        }
    }
}
