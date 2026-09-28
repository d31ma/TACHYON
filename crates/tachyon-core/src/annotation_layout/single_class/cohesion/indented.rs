use super::method::Method;
use super::words::Words;
use std::ops::Range;

pub(super) struct IndentedMembers<'r, 'a> {
    words: &'r Words<'a>,
    methods: Vec<Method>,
}

impl<'r, 'a> IndentedMembers<'r, 'a> {
    pub(super) fn read(words: &'r Words<'a>) -> Vec<Method> {
        let mut reader = Self {
            words,
            methods: Vec::new(),
        };
        reader.read_all();
        reader.methods
    }

    fn read_all(&mut self) {
        let mut at = 0;
        while at < self.words.len() {
            at = self.read_member(at);
        }
    }

    fn read_member(&mut self, at: usize) -> usize {
        let is_method = (self.words.text(at) == "def")
            & (self.words.column(at) > 0)
            & !self.words.after_member_access(at);
        if !is_method {
            return at + 1;
        }
        let method = self.method(at);
        let resume = method.body.end.max(at + 1);
        self.methods.push(method);
        resume
    }

    fn method(&self, at: usize) -> Method {
        let name = self.words.text(at + 1);
        let dunder = name.starts_with("__") & name.ends_with("__");
        Method {
            name: name.to_owned(),
            head: at,
            public: !name.starts_with('_') | dunder,
            body: self.body(at),
            receiver: String::new(),
            nested: Vec::new(),
        }
    }

    fn body(&self, at: usize) -> Range<usize> {
        let colon = self
            .words
            .find_at_depth(at + 2, self.words.depth(at), &[":"])
            .unwrap_or(at + 1);
        let start = colon + 1;
        start
            ..self
                .words
                .block_end(start, self.words.column(at))
                .max(start)
    }
}
