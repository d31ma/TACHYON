use super::CppToken;

pub(crate) struct CppLexer<'a> {
    code: &'a [u8],
    at: usize,
    tokens: Vec<CppToken>,
}

impl<'a> CppLexer<'a> {
    const PAIRS: [&'static str; 2] = ["::", "->"];

    pub(crate) fn tokens(code: &'a str) -> Vec<CppToken> {
        let lexer = Self {
            code: code.as_bytes(),
            at: 0,
            tokens: Vec::new(),
        };
        lexer.run()
    }

    fn run(mut self) -> Vec<CppToken> {
        while self.at < self.code.len() {
            self.step();
        }
        self.tokens
    }

    fn step(&mut self) {
        let length = self.token_length();
        let start = self.at;
        self.at += length.max(1);
        self.keep(start, length);
    }

    fn keep(&mut self, start: usize, length: usize) {
        if length == 0 {
            return;
        }
        let text = String::from_utf8_lossy(&self.code[start..start + length]).into_owned();
        self.tokens.push(CppToken { text, start });
    }

    fn token_length(&self) -> usize {
        let byte = self.code[self.at];
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'_' | b'0'..=b'9' => self.word_length(),
            byte if byte.is_ascii_whitespace() => 0,
            _ => self.symbol_length(),
        }
    }

    fn word_length(&self) -> usize {
        self.code[self.at..]
            .iter()
            .take_while(|byte| byte.is_ascii_alphanumeric() || **byte == b'_' || **byte == b'.')
            .count()
    }

    fn symbol_length(&self) -> usize {
        let rest = &self.code[self.at..];
        let paired = Self::PAIRS
            .iter()
            .any(|pair| rest.starts_with(pair.as_bytes()));
        1 + usize::from(paired)
    }
}

#[cfg(test)]
mod tests {
    use super::CppLexer;

    fn texts(code: &str) -> Vec<String> {
        CppLexer::tokens(code)
            .into_iter()
            .map(|token| token.text)
            .collect()
    }

    #[test]
    fn words_numbers_and_symbols_become_tokens() {
        assert_eq!(
            texts("class [[Service]] A { std::vector<int> b = 1.5; };"),
            [
                "class", "[", "[", "Service", "]", "]", "A", "{", "std", "::", "vector", "<",
                "int", ">", "b", "=", "1.5", ";", "}", ";"
            ]
        );
    }

    #[test]
    fn a_token_knows_where_it_starts() {
        let tokens = CppLexer::tokens("  int x;");
        assert_eq!(tokens[1].start, 6);
        assert!(!tokens[0].touches(&tokens[1]));
    }
}
