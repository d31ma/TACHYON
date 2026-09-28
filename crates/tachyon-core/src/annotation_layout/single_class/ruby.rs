use super::{Failure, Parser};

impl Parser<'_, '_> {
    pub(super) fn read_ruby_import(&mut self) -> Result<(), Failure> {
        let start = self.at;
        self.at += 1;
        let parenthesized = self.word(self.at) == "(";
        if parenthesized {
            self.at += 1;
        }
        let names_file = self
            .tokens
            .items
            .get(self.at)
            .is_some_and(|token| token.literal);
        if !names_file {
            return Err(self.refused(start, "a require without a file name in quotes"));
        }
        self.at += 1;
        if parenthesized {
            if self.word(self.at) != ")" {
                return Err(self.refused(start, "an unclosed require"));
            }
            self.at += 1;
        }
        if self.word(self.at) == ";" {
            self.at += 1;
        }
        let line_ends = self.at == self.tokens.items.len() || self.tokens.new_line_before(self.at);
        if !line_ends {
            return Err(self.refused(self.at, "code after a require"));
        }
        Ok(())
    }

    pub(super) fn read_ruby_body(&mut self, start: usize, keyword: usize) -> Result<(), Failure> {
        let column = self.tokens.column(keyword);
        let first_line_end = self.line_end_token(keyword);
        while self.at < self.tokens.items.len() {
            let closes_one_line_class = self.at + 1 == first_line_end;
            let closes_at_column =
                self.tokens.new_line_before(self.at) && self.tokens.column(self.at) == column;
            if self.word(self.at) == "end" && (closes_one_line_class || closes_at_column) {
                self.at += 1;
                return Ok(());
            }
            self.at += 1;
        }
        Err(self.refused(
            start,
            "a class whose `end` is not at the same indentation as its `class` line",
        ))
    }

    fn line_end_token(&self, at: usize) -> usize {
        let mut next = at + 1;
        while next < self.tokens.items.len() && !self.tokens.new_line_before(next) {
            next += 1;
        }
        next
    }
}
