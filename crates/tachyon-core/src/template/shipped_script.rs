pub(crate) struct ShippedScript<'a> {
    source: &'a [u8],
    position: usize,
    output: Vec<u8>,
}

impl ShippedScript<'_> {
    const NEW_LINE: u8 = b'\n';
    const SPACE: u8 = b' ';
    const TAB: u8 = b'\t';
    const BACKSLASH: u8 = b'\\';
    const SLASH: u8 = b'/';
    const STAR: u8 = b'*';
    const DOT: u8 = b'.';
    const BACKTICK: u8 = b'`';
    const DOLLAR: u8 = b'$';
    const OPEN_BRACE: u8 = b'{';
    const CLOSE_BRACE: u8 = b'}';
    const OPEN_CLASS: u8 = b'[';
    const CLOSE_CLASS: u8 = b']';
    const QUOTES: [u8; 2] = *b"'\"";
    const VALUE_ENDS: [u8; 7] = *b")]}'\"`/";
    const WORDS_BEFORE_VALUES: [&'static [u8]; 14] = [
        b"return",
        b"typeof",
        b"instanceof",
        b"in",
        b"of",
        b"new",
        b"delete",
        b"void",
        b"throw",
        b"case",
        b"do",
        b"else",
        b"yield",
        b"await",
    ];

    pub(crate) fn compact(source: &str) -> String {
        let mut script = ShippedScript {
            source: source.as_bytes(),
            position: 0,
            output: Vec::with_capacity(source.len()),
        };
        script.skip_indent();
        while script.position < script.source.len() {
            script.code();
        }
        String::from_utf8(script.output).unwrap_or_else(|_| source.to_owned())
    }

    fn code(&mut self) {
        let mut depth = 0_usize;
        while let Some(byte) = self.peek(0) {
            match byte {
                Self::NEW_LINE => self.line_break(),
                Self::SPACE | Self::TAB => self.space(),
                Self::BACKTICK => self.template(),
                Self::SLASH => self.slash(),
                Self::OPEN_BRACE => {
                    depth += 1;
                    self.copy(1);
                }
                Self::CLOSE_BRACE if depth == 0 => {
                    self.copy(1);
                    return;
                }
                Self::CLOSE_BRACE => {
                    depth -= 1;
                    self.copy(1);
                }
                quote if Self::QUOTES.contains(&quote) => self.quoted(quote),
                _ => self.copy(1),
            }
        }
    }

    fn line_break(&mut self) {
        self.position += 1;
        self.end_line();
        self.skip_indent();
    }

    fn end_line(&mut self) {
        self.trim_trailing_space();
        if !matches!(self.output.last(), None | Some(&Self::NEW_LINE)) {
            self.output.push(Self::NEW_LINE);
        }
    }

    fn space(&mut self) {
        self.position += 1;
        self.separate();
    }

    fn separate(&mut self) {
        if !matches!(
            self.output.last(),
            None | Some(&(Self::SPACE | Self::NEW_LINE))
        ) {
            self.output.push(Self::SPACE);
        }
    }

    fn slash(&mut self) {
        match self.peek(1) {
            Some(Self::STAR) => self.block_comment(),
            Some(Self::SLASH) => self.line_comment(),
            _ if self.starts_value() => self.regular_expression(),
            _ => self.copy(1),
        }
    }

    fn block_comment(&mut self) {
        let start = self.position;
        let end = self
            .find(b"*/", start + 2)
            .map_or(self.source.len(), |found| found + 2);
        self.position = end;
        if self.source[start..end].contains(&Self::NEW_LINE) {
            self.end_line();
            self.skip_indent();
        } else {
            self.separate();
        }
    }

    fn line_comment(&mut self) {
        while !matches!(self.peek(0), None | Some(Self::NEW_LINE)) {
            self.position += 1;
        }
    }

    fn regular_expression(&mut self) {
        self.copy(1);
        let mut in_class = false;
        while let Some(byte) = self.peek(0) {
            match byte {
                Self::BACKSLASH => self.copy(2),
                Self::OPEN_CLASS => {
                    in_class = true;
                    self.copy(1);
                }
                Self::CLOSE_CLASS => {
                    in_class = false;
                    self.copy(1);
                }
                Self::SLASH if !in_class => {
                    self.copy(1);
                    return;
                }
                Self::NEW_LINE => return,
                _ => self.copy(1),
            }
        }
    }

    fn quoted(&mut self, quote: u8) {
        self.copy(1);
        while let Some(byte) = self.peek(0) {
            match byte {
                Self::BACKSLASH => self.copy(2),
                found if found == quote => {
                    self.copy(1);
                    return;
                }
                _ => self.copy(1),
            }
        }
    }

    fn template(&mut self) {
        self.copy(1);
        while let Some(byte) = self.peek(0) {
            match byte {
                Self::BACKSLASH => self.copy(2),
                Self::BACKTICK => {
                    self.copy(1);
                    return;
                }
                Self::DOLLAR if self.peek(1) == Some(Self::OPEN_BRACE) => {
                    self.copy(2);
                    self.code();
                }
                _ => self.copy(1),
            }
        }
    }

    fn starts_value(&self) -> bool {
        let Some(&last) = self
            .output
            .iter()
            .rev()
            .find(|byte| !byte.is_ascii_whitespace())
        else {
            return true;
        };
        if Self::is_word_byte(last) {
            return self.ends_with_value_word();
        }
        !Self::VALUE_ENDS.contains(&last)
    }

    fn ends_with_value_word(&self) -> bool {
        let trimmed = self.output.trim_ascii_end();
        let start = trimmed
            .iter()
            .rposition(|byte| !Self::is_word_byte(*byte))
            .map_or(0, |found| found + 1);
        let property = start > 0 && trimmed[start - 1] == Self::DOT;
        !property && Self::WORDS_BEFORE_VALUES.contains(&&trimmed[start..])
    }

    fn is_word_byte(byte: u8) -> bool {
        byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$' | b'#')
    }

    fn skip_indent(&mut self) {
        while matches!(self.peek(0), Some(Self::SPACE | Self::TAB)) {
            self.position += 1;
        }
    }

    fn trim_trailing_space(&mut self) {
        while matches!(self.output.last(), Some(&Self::SPACE | &Self::TAB)) {
            self.output.pop();
        }
    }

    fn copy(&mut self, count: usize) {
        let end = (self.position + count).min(self.source.len());
        self.output
            .extend_from_slice(&self.source[self.position..end]);
        self.position = end;
    }

    fn peek(&self, offset: usize) -> Option<u8> {
        self.source.get(self.position + offset).copied()
    }

    fn find(&self, needle: &[u8], from: usize) -> Option<usize> {
        self.source
            .get(from..)?
            .windows(needle.len())
            .position(|window| window == needle)
            .map(|found| found + from)
    }
}
