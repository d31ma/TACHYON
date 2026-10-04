use super::CppTokens;

pub(crate) struct CppMemberEnd<'a> {
    tokens: &'a CppTokens,
    at: usize,
    parameters: bool,
    equals: bool,
    initializers: bool,
    angles: usize,
}

impl<'a> CppMemberEnd<'a> {
    pub(crate) fn find(tokens: &'a CppTokens, start: usize, limit: usize) -> (usize, usize) {
        let mut scan = Self {
            tokens,
            at: start,
            parameters: false,
            equals: false,
            initializers: false,
            angles: 0,
        };
        scan.scan(limit)
    }

    fn scan(&mut self, limit: usize) -> (usize, usize) {
        let mut found = None;
        while [found.is_none(), self.at < limit] == [true, true] {
            found = self.step();
        }
        found.unwrap_or((limit, limit))
    }

    fn step(&mut self) -> Option<(usize, usize)> {
        let at = self.at;
        self.at = self.tokens.skipped(at);
        match self.tokens.text(at) {
            ";" => Some((at + 1, at)),
            "=" => self.note_equals(),
            "<" => self.note_angles(1, 0),
            ">" => self.note_angles(0, 1),
            ":" => self.note_initializers(),
            "(" => self.note_parameters(),
            "{" => self.brace(at),
            _ => None,
        }
    }

    fn note_equals(&mut self) -> Option<(usize, usize)> {
        self.equals |= self.angles == 0;
        None
    }

    fn note_angles(&mut self, opened: usize, closed: usize) -> Option<(usize, usize)> {
        self.angles = (self.angles + opened).saturating_sub(closed);
        None
    }

    fn note_initializers(&mut self) -> Option<(usize, usize)> {
        self.initializers |= [self.parameters, !self.equals] == [true, true];
        None
    }

    fn note_parameters(&mut self) -> Option<(usize, usize)> {
        self.parameters |= [self.angles == 0, !self.equals] == [true, true];
        None
    }

    fn brace(&self, open: usize) -> Option<(usize, usize)> {
        self.starts_body(open)
            .then(|| (self.after_body(open), open))
    }

    fn starts_body(&self, open: usize) -> bool {
        let after_parameters = [self.parameters, !self.equals] == [true, true];
        let previous = open.saturating_sub(1);
        let initializer_brace =
            [self.tokens.is(previous, ">"), self.tokens.is_word(previous)].contains(&true);
        let member_initializer = [self.initializers, initializer_brace] == [true, true];
        [after_parameters, !member_initializer] == [true, true]
    }

    fn after_body(&self, open: usize) -> usize {
        let close = self.tokens.partner(open).unwrap_or(open);
        close + 1 + usize::from(self.tokens.is(close + 1, ";"))
    }
}
