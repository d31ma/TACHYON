use super::words::Words;
use std::ops::Range;

pub(super) struct SwiftGuardReturn<'r, 'a> {
    words: &'r Words<'a>,
}

impl<'r, 'a> SwiftGuardReturn<'r, 'a> {
    const GUARD: &'static str = "guard";
    const BINDINGS: &'static [&'static str] = &["let", "var"];
    const ELSE: &'static [&'static str] = &["else"];
    const LOGICAL_JOINERS: &'static [&'static str] = &["&&", "||"];
    const LIST_JOINER: &'static str = ",";
    const OPEN: &'static str = "{";
    const RETURN: &'static str = "return";
    const LONGEST_BODY: usize = 2;

    pub(super) fn new(words: &'r Words<'a>) -> Self {
        Self { words }
    }

    pub(super) fn matches(&self, at: usize) -> bool {
        let depth = self.words.depth(at);
        let otherwise = self.words.find_at_depth(at + 1, depth, Self::ELSE);
        let open = otherwise.map_or(at, |word| word + 1);
        (self.words.text(at) == Self::GUARD)
            & self.words.line_start(at)
            & Self::BINDINGS.contains(&self.words.text(at + 1))
            & !self.joins_conditions(at + 1..open, depth)
            & (self.words.text(open) == Self::OPEN)
            & self.returns_plainly(self.words.inside(open))
    }

    fn joins_conditions(&self, condition: Range<usize>, depth: usize) -> bool {
        condition.clone().any(|at| self.joins(at, depth))
    }

    fn joins(&self, at: usize, depth: usize) -> bool {
        let logical = Self::LOGICAL_JOINERS.contains(&self.words.pair(at));
        let listed = (self.words.text(at) == Self::LIST_JOINER) & (self.words.depth(at) == depth);
        logical | listed
    }

    fn returns_plainly(&self, body: Range<usize>) -> bool {
        let length = body.end.saturating_sub(body.start);
        let plain_value = (length == 1) | self.words.is_simple_value(body.start + 1);
        (self.words.text(body.start) == Self::RETURN) & (length <= Self::LONGEST_BODY) & plain_value
    }
}
