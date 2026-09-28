use crate::lexical::LexicalMask;

type LexicalError = (usize, &'static str);

pub(crate) struct FsharpLexer<'a> {
    bytes: &'a [u8],
    code: Vec<bool>,
    comment: Vec<bool>,
}

impl<'a> FsharpLexer<'a> {
    const UNTERMINATED_STRING: &'static str = "Unterminated F# string";
    const UNTERMINATED_COMMENT: &'static str = "Unterminated F# block comment";
    const TRIPLE_QUOTE: &'static [u8] = b"\"\"\"";
    const LONGEST_CHARACTER: usize = 12;
    const COMMENT_OPENERS: [&'static [u8]; 2] = [b"//", b"(*"];

    pub(crate) fn code_mask(source: &'a str) -> LexicalMask {
        let (lexer, error) = Self::scanned(source);
        lexer.into_mask(error)
    }

    pub(crate) fn without_comments(source: &'a str) -> String {
        let (lexer, _) = Self::scanned(source);
        String::from_utf8(lexer.comments_blanked()).unwrap_or_else(|_| source.to_owned())
    }

    fn comments_blanked(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.bytes.len());
        for (byte, comment) in self.bytes.iter().zip(&self.comment) {
            bytes.push(Self::blanked(*byte, *comment));
        }
        bytes
    }

    fn scanned(source: &'a str) -> (Self, Option<LexicalError>) {
        let mut lexer = Self {
            bytes: source.as_bytes(),
            code: vec![true; source.len()],
            comment: vec![false; source.len()],
        };
        let error = lexer.scan().err();
        (lexer, error)
    }

    fn blanked(byte: u8, comment: bool) -> u8 {
        if comment && byte != b'\n' {
            return b' ';
        }
        byte
    }

    fn into_mask(self, error: Option<LexicalError>) -> LexicalMask {
        let mut code = self.code;
        let Some((start, message)) = error else {
            return LexicalMask {
                code,
                error: None,
                marker_problem: None,
            };
        };
        code[start..].fill(false);
        LexicalMask {
            code,
            error: Some(message),
            marker_problem: None,
        }
    }

    fn scan(&mut self) -> Result<(), LexicalError> {
        let mut at = 0;
        while at < self.bytes.len() {
            at = self.step(at)?;
        }
        Ok(())
    }

    fn step(&mut self, at: usize) -> Result<usize, LexicalError> {
        let end = self.region_end(at)?.unwrap_or(at);
        self.code[at..end].fill(false);
        self.comment[at..end].fill(Self::starts_comment(&self.bytes[at..]));
        Ok(end.max(at + 1))
    }

    fn region_end(&self, at: usize) -> Result<Option<usize>, LexicalError> {
        let rest = &self.bytes[at..];
        if rest.starts_with(b"//") {
            Ok(Some(self.line_end(at)))
        } else if rest.starts_with(b"(*)") {
            Ok(None)
        } else if rest.starts_with(b"(*") {
            Self::closed(at, self.block_comment_end(at), Self::UNTERMINATED_COMMENT)
        } else if rest.starts_with(Self::TRIPLE_QUOTE) {
            Self::closed(at, self.triple_end(at + 3), Self::UNTERMINATED_STRING)
        } else if rest.starts_with(b"$\"\"\"") {
            Self::closed(at, self.triple_end(at + 4), Self::UNTERMINATED_STRING)
        } else if rest.starts_with(b"@\"") || rest.starts_with(b"$@\"") || rest.starts_with(b"@$\"")
        {
            Self::closed(at, self.verbatim_end(at), Self::UNTERMINATED_STRING)
        } else if rest.starts_with(b"$\"") {
            Self::closed(at, self.escaped_end(at + 1), Self::UNTERMINATED_STRING)
        } else if rest.starts_with(b"\"") {
            Self::closed(at, self.escaped_end(at), Self::UNTERMINATED_STRING)
        } else if rest.starts_with(b"'") {
            Ok(self.character_end(at))
        } else {
            Ok(None)
        }
    }

    fn starts_comment(rest: &[u8]) -> bool {
        let opens = Self::COMMENT_OPENERS
            .iter()
            .any(|opener| rest.starts_with(opener));
        opens && !rest.starts_with(b"(*)")
    }

    fn closed(
        at: usize,
        end: Option<usize>,
        message: &'static str,
    ) -> Result<Option<usize>, LexicalError> {
        end.map(Some).ok_or((at, message))
    }

    fn line_end(&self, at: usize) -> usize {
        self.bytes[at..]
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(self.bytes.len(), |offset| at + offset)
    }

    fn block_comment_end(&self, at: usize) -> Option<usize> {
        let mut depth = 1;
        let mut position = at + 2;
        while depth > 0 && position < self.bytes.len() {
            (position, depth) = self.comment_step(position, depth);
        }
        (depth == 0).then_some(position)
    }

    fn comment_step(&self, at: usize, depth: usize) -> (usize, usize) {
        let rest = &self.bytes[at..];
        if rest.starts_with(b"*)") {
            (at + 2, depth - 1)
        } else if rest.starts_with(b"(*") && !rest.starts_with(b"(*)") {
            (at + 2, depth + 1)
        } else {
            (at + 1, depth)
        }
    }

    fn triple_end(&self, from: usize) -> Option<usize> {
        let offset = self.bytes[from..]
            .windows(Self::TRIPLE_QUOTE.len())
            .position(|window| window == Self::TRIPLE_QUOTE)?;
        Some(from + offset + Self::TRIPLE_QUOTE.len())
    }

    fn verbatim_end(&self, at: usize) -> Option<usize> {
        let quote = at + self.bytes[at..].iter().position(|byte| *byte == b'"')?;
        self.string_end(quote + 1, Self::verbatim_step)
    }

    fn escaped_end(&self, quote: usize) -> Option<usize> {
        self.string_end(quote + 1, Self::escaped_step)
    }

    fn string_end(&self, from: usize, step: fn(&Self, usize) -> (usize, bool)) -> Option<usize> {
        let mut position = from;
        let mut closed = false;
        while !closed && position < self.bytes.len() {
            (position, closed) = step(self, position);
        }
        closed.then_some(position)
    }

    fn verbatim_step(&self, at: usize) -> (usize, bool) {
        let rest = &self.bytes[at..];
        if rest.starts_with(b"\"\"") {
            (at + 2, false)
        } else if rest.starts_with(b"\"") {
            (at + 1, true)
        } else {
            (at + 1, false)
        }
    }

    fn escaped_step(&self, at: usize) -> (usize, bool) {
        match self.bytes[at] {
            b'\\' => (at + 2, false),
            b'"' => (at + 1, true),
            _ => (at + 1, false),
        }
    }

    fn follows_word(&self, at: usize) -> bool {
        at.checked_sub(1)
            .map(|before| self.bytes[before])
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    }

    fn character_end(&self, at: usize) -> Option<usize> {
        Some(at).filter(|quote| !self.follows_word(*quote))?;
        let escaped = self.bytes.get(at + 1) == Some(&b'\\');
        let search = (at + 2 + usize::from(escaped)).min(self.bytes.len());
        let limit = self
            .bytes
            .len()
            .min(at + Self::LONGEST_CHARACTER)
            .max(search);
        let closing = search
            + self.bytes[search..limit]
                .iter()
                .position(|byte| *byte == b'\'')?;
        let inside = &self.bytes[at + 1..closing];
        Self::is_character(inside).then_some(closing + 1)
    }

    fn is_character(inside: &[u8]) -> bool {
        match inside.first() {
            Some(b'\\') => inside.len() > 1,
            Some(_) => std::str::from_utf8(inside).is_ok_and(|text| text.chars().count() == 1),
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FsharpLexer;

    fn visible(source: &str) -> String {
        let mask = FsharpLexer::code_mask(source);
        source
            .char_indices()
            .map(|(at, character)| if mask.code[at] { character } else { ' ' })
            .collect()
    }

    #[test]
    fn comments_and_strings_are_hidden() {
        let source = "let a = \"x // y\" // note\n(* block (* nested *) *) let b = 1";
        assert_eq!(
            visible(source),
            "let a =                 \n                         let b = 1"
        );
    }

    #[test]
    fn the_multiply_operator_is_not_a_comment() {
        assert_eq!(visible("let f = (*) 2"), "let f = (*) 2");
    }

    #[test]
    fn verbatim_triple_and_interpolated_strings_are_hidden() {
        let source = "a @\"c:\\\"\"x\" b \"\"\"q \" r\"\"\" c $\"{n} \\\" \" d";
        assert_eq!(visible(source), "a           b             c            d");
    }

    #[test]
    fn character_literals_are_hidden_but_type_variables_and_primes_stay() {
        let source = "let c = 'x' in f '\\n' '\\'' ('T list) x' 'a";
        assert_eq!(
            visible(source),
            "let c =     in f           ('T list) x' 'a"
        );
    }

    #[test]
    fn comments_become_spaces_and_strings_stay() {
        let source = "let a = \"(* x *)\" (* note\n more *) // end\nb";
        assert_eq!(
            FsharpLexer::without_comments(source),
            "let a = \"(* x *)\"        \n               \nb"
        );
    }

    #[test]
    fn an_unterminated_string_is_an_error() {
        let mask = FsharpLexer::code_mask("let a = \"open");
        assert_eq!(mask.error, Some("Unterminated F# string"));
        let mask = FsharpLexer::code_mask("(* open");
        assert_eq!(mask.error, Some("Unterminated F# block comment"));
    }
}
