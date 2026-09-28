use super::{Failure, Parser};

impl Parser<'_, '_> {
    pub(super) fn skip_go_package(&mut self) -> Result<(), Failure> {
        if self.input.extension != "go" || self.word(self.at) != "package" {
            return Ok(());
        }
        if self.word(self.at + 1) != "main" {
            return Err(self.input.failure(
                self.position(self.at),
                "A Go handler file is compiled as package main; found another package clause",
                "Write `package main` as the first line, or leave the package clause out.",
            ));
        }
        self.at += 2;
        self.skip_go_semicolon();
        Ok(())
    }

    pub(super) fn read_go_import(&mut self) -> Result<(), Failure> {
        let start = self.at;
        self.at += 1;
        if self.word(self.at) != "(" {
            return self.read_go_import_path();
        }
        let end = self
            .tokens
            .after_group(self.at)
            .ok_or_else(|| self.refused(start, "an unclosed import"))?;
        self.at += 1;
        while self.at + 1 < end {
            self.read_go_import_path()?;
        }
        self.at = end;
        self.skip_go_semicolon();
        Ok(())
    }

    fn read_go_import_path(&mut self) -> Result<(), Failure> {
        if self.is_name(self.at) || self.word(self.at) == "." {
            self.at += 1;
        }
        let is_path = self
            .tokens
            .items
            .get(self.at)
            .is_some_and(|token| token.literal);
        if !is_path {
            return Err(self.refused(self.at, "code outside an import or type"));
        }
        self.at += 1;
        self.skip_go_semicolon();
        Ok(())
    }

    pub(super) fn read_go_declaration(
        &mut self,
        start: usize,
        markers: &[(String, usize)],
    ) -> Result<(), Failure> {
        match self.word(self.at) {
            "type" => self.read_go_type(start, markers),
            "func" => self.read_go_method(start, markers),
            _ => Err(self.refused_top_level()),
        }
    }

    fn read_go_type(&mut self, start: usize, markers: &[(String, usize)]) -> Result<(), Failure> {
        self.at += 1;
        if self.word(self.at) == "(" {
            return Err(self.refused(start, "a grouped type declaration"));
        }
        let name = self.read_name()?;
        if self.word(self.at) == "[" {
            self.at = self
                .tokens
                .after_group(self.at)
                .ok_or_else(|| self.refused(start, "an unclosed type parameter list"))?;
        }
        if self.word(self.at) == "=" {
            return Err(self.refused(start, &format!("a type alias '{name}'")));
        }
        if self.declared.is_some() {
            let kind = match self.word(self.at) {
                "struct" | "interface" => self.word(self.at).to_owned(),
                _ => String::from("type"),
            };
            return Err(self.refused(start, &Self::another_type(&kind, &name, markers)));
        }
        self.check_roles(markers, &name)?;
        self.declared = Some(name);
        self.skip_go_line(start)
    }

    fn read_go_method(&mut self, start: usize, markers: &[(String, usize)]) -> Result<(), Failure> {
        self.at += 1;
        if self.word(self.at) != "(" {
            let name = self.word(self.at).chars().take(128).collect::<String>();
            return Err(self.refused(start, &format!("top-level 'func {name}'")));
        }
        let receiver_end = self
            .tokens
            .after_group(self.at)
            .ok_or_else(|| self.refused(start, "an unclosed method receiver"))?;
        let receiver = self.go_receiver_type(self.at + 1, receiver_end - 1)?;
        self.at = receiver_end;
        if !self.is_name(self.at) {
            return Err(self.refused(start, "a method without a name"));
        }
        self.at += 1;
        self.check_roles(markers, &receiver)?;
        self.implementations.push((receiver, start));
        self.skip_go_method_body(start)
    }

    fn go_receiver_type(&self, first: usize, end: usize) -> Result<String, Failure> {
        let mut name = None;
        let mut at = first;
        while at < end && self.word(at) != "[" {
            if self.is_name(at) {
                name = Some(self.word(at));
            } else if self.word(at) != "*" {
                return Err(self.refused(at, "an invalid method receiver"));
            }
            at += 1;
        }
        let Some(name) = name else {
            return Err(self.refused(first, "a method without a receiver type"));
        };
        Ok(name.to_owned())
    }

    fn skip_go_method_body(&mut self, start: usize) -> Result<(), Failure> {
        while self.at < self.tokens.items.len() && !self.tokens.new_line_before(self.at) {
            let opens_body = self.word(self.at) == "{"
                && !matches!(self.word(self.at - 1), "struct" | "interface");
            if matches!(self.word(self.at), "(" | "[" | "{") {
                self.at = self
                    .tokens
                    .after_group(self.at)
                    .ok_or_else(|| self.refused(start, "an unclosed method"))?;
            } else {
                self.at += 1;
            }
            if opens_body {
                self.skip_go_semicolon();
                return Ok(());
            }
        }
        Err(self.refused(start, "a method without a body"))
    }

    fn skip_go_line(&mut self, start: usize) -> Result<(), Failure> {
        while self.at < self.tokens.items.len() && !self.tokens.new_line_before(self.at) {
            if self.word(self.at) == ";" {
                self.at += 1;
                return Ok(());
            }
            if matches!(self.word(self.at), "(" | "[" | "{") {
                self.at = self
                    .tokens
                    .after_group(self.at)
                    .ok_or_else(|| self.refused(start, "an unclosed type"))?;
            } else {
                self.at += 1;
            }
        }
        Ok(())
    }

    fn skip_go_semicolon(&mut self) {
        if self.word(self.at) == ";" {
            self.at += 1;
        }
    }
}
