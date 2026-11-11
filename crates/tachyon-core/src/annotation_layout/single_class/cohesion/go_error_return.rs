use super::words::Words;
use std::ops::Range;

pub(super) struct GoErrorReturn<'r, 'a> {
    words: &'r Words<'a>,
}

impl<'r, 'a> GoErrorReturn<'r, 'a> {
    const CHECK: [&'static str; 6] = ["if", "err", "!", "=", "nil", "{"];
    const NOT_EQUAL: &'static str = "!=";
    const WRAP_HEAD: [&'static str; 4] = ["fmt", ".", "Errorf", "("];
    const WRAP_TAIL: [&'static str; 3] = [",", "err", ")"];
    const WRAP_LENGTH: usize = 8;
    const WRAP_VERB: &'static str = "%w";
    const ERROR_NAME: &'static str = "err";
    const RETURN: &'static str = "return";
    const VALUE_STARTS: &'static [&'static str] = &["return", ","];

    pub(super) fn new(words: &'r Words<'a>) -> Self {
        Self { words }
    }

    pub(super) fn matches(&self, at: usize) -> bool {
        let open = at + Self::CHECK.len() - 1;
        let body = self.words.inside(open);
        self.checks_error(at)
            & self.returns_only_error(body)
            & (self.words.text(self.words.after_group(open)) != "else")
            & self.follows_error_assignment(at)
    }

    fn checks_error(&self, at: usize) -> bool {
        self.words.follows(at, &Self::CHECK)
            & self.words.line_start(at)
            & (self.words.pair(at + 2) == Self::NOT_EQUAL)
    }

    fn returns_only_error(&self, body: Range<usize>) -> bool {
        let value = self.error_value_start(body.end.wrapping_sub(1));
        (self.words.text(body.start) == Self::RETURN)
            & self.is_single_statement(body)
            & Self::VALUE_STARTS.contains(&self.words.text(value.wrapping_sub(1)))
    }

    fn error_value_start(&self, last: usize) -> usize {
        if self.words.text(last) == Self::ERROR_NAME {
            return last;
        }
        self.wrapped_error_start(last.wrapping_sub(Self::WRAP_LENGTH - 1))
    }

    fn wrapped_error_start(&self, start: usize) -> usize {
        let message = start.wrapping_add(Self::WRAP_HEAD.len());
        let wrapped = self.words.follows(start, &Self::WRAP_HEAD)
            & self.words.is_literal(message)
            & self.words.text(message).contains(Self::WRAP_VERB)
            & self
                .words
                .follows(message.wrapping_add(1), &Self::WRAP_TAIL);
        if wrapped {
            return start;
        }
        usize::MAX
    }

    fn is_single_statement(&self, body: Range<usize>) -> bool {
        let depth = self.words.depth(body.start);
        !(body.start + 1..body.end).any(|at| self.starts_statement(at, depth))
    }

    fn starts_statement(&self, at: usize, depth: usize) -> bool {
        let placed = self.words.line_start(at) | (self.words.text(at) == ";");
        placed & (self.words.depth(at) == depth)
    }

    fn follows_error_assignment(&self, at: usize) -> bool {
        let depth = self.words.depth(at);
        let start = self.words.statement_start(at, depth);
        (start..at).any(|word| self.assigns_error(word, depth))
    }

    fn assigns_error(&self, at: usize, depth: usize) -> bool {
        let next = at + 1;
        let declares = self.words.pair(next) == ":=";
        let assigns = (self.words.text(next) == "=") & (self.words.pair(next) != "==");
        (self.words.text(at) == Self::ERROR_NAME)
            & (self.words.depth(at) == depth)
            & (declares | assigns)
    }
}
