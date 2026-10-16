use crate::Failure;
use crate::ascii_name::AsciiName;
use crate::go_source::GoMarkerName;
use crate::lexical::LexicalMask;
pub(crate) use needed_field::NeededField;
pub(crate) use ruby_needs::RubyNeeds;
use std::path::Path;

mod needed_field;
mod ruby_needs;

pub(crate) struct RubyMarkers;

impl RubyMarkers {
    const MAX_NAME: usize = 128;
    const HELP: &'static str =
        "Write each Ruby annotation as `#[Name]` on its own line before the declaration.";

    pub(crate) fn validate(path: &Path, source: &str, code: &str) -> Result<(), Failure> {
        if let Some((at, message)) = RubyLexer::code_mask(source).marker_problem {
            return Err(Self::failure(path, source, at, message));
        }
        let mut offset = 0;
        for line in code.split_inclusive('\n') {
            let trimmed = line.trim_start();
            let at = offset + line.len() - trimmed.len();
            offset += line.len();
            if let Some(rest) = trimmed.strip_prefix("#[") {
                if let Some(message) = Self::refusal(GoMarkerName::read(rest)) {
                    return Err(Self::failure(path, source, at, message));
                }
                continue;
            }
            if let Some(rest) = trimmed.strip_prefix('@')
                && Self::is_bare_marker(rest)
            {
                let name = GoMarkerName::read(rest);
                let message = format!(
                    "Ruby annotations are comments: write `#[{name}]` on its own line, not `@{name}`"
                );
                return Err(Self::failure(path, source, at, message));
            }
        }
        Ok(())
    }

    fn is_bare_marker(rest: &str) -> bool {
        let name = GoMarkerName::read(rest);
        let after = rest[name.len()..].trim_end();
        let whole_line = after.is_empty() || (after.starts_with('(') && after.ends_with(')'));
        let capitalized = name.as_bytes().first().is_some_and(u8::is_ascii_uppercase);
        whole_line && capitalized && GoMarkerName::known(name)
    }

    fn refusal(name: &str) -> Option<String> {
        if GoMarkerName::known(name) {
            return None;
        }
        let written: String = name.chars().take(Self::MAX_NAME).collect();
        let (namespace, plain) = match written.rsplit_once("::") {
            Some((namespace, plain)) => (format!("{namespace}::"), plain.to_owned()),
            None => (String::new(), written.clone()),
        };
        let lowered = namespace.to_ascii_lowercase();
        if namespace != lowered {
            return Some(format!(
                "unknown annotation `#[{written}]`; namespaces are lowercase: write `#[{lowered}{plain}]`"
            ));
        }
        let help = GoMarkerName::suggestion(&plain).map_or_else(String::new, |candidate| {
            format!("; did you mean `#[{namespace}{candidate}]`?")
        });
        Some(format!("unknown annotation `#[{written}]`{help}"))
    }

    fn failure(path: &Path, source: &str, at: usize, message: impl Into<String>) -> Failure {
        Failure::one(crate::failure::diagnostic(
            1308,
            message,
            Some(String::from(Self::HELP)),
            crate::failure::source_span(&path.to_string_lossy(), at, (at + 1).min(source.len())),
        ))
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct RubyMembers {
    pub(crate) fields: Vec<String>,
    pub(crate) writable: Vec<String>,
    pub(crate) methods: Vec<String>,
}

impl RubyMembers {
    const HIDING_SECTIONS: [&'static str; 2] = ["private", "protected"];

    pub(crate) fn without_needed(mut self, source: &str) -> Self {
        let needed = RubyNeeds::fields(source);
        self.fields.retain(|field| !needed.contains(field));
        self.writable.retain(|field| !needed.contains(field));
        self
    }

    pub(crate) fn read(code: &str) -> Self {
        let mut members = Self::default();
        let mut member_indent = None;
        let mut hidden = false;
        for line in code.lines() {
            let text = line.trim();
            if text.is_empty() {
                continue;
            }
            let indent = line.len() - line.trim_start().len();
            if indent == 0 {
                hidden = false;
                member_indent = None;
                continue;
            }
            if indent != *member_indent.get_or_insert(indent) {
                continue;
            }
            if Self::HIDING_SECTIONS.contains(&text) {
                hidden = true;
            } else if text == "public" {
                hidden = false;
            } else if !hidden {
                members.read_member(text);
            }
        }
        members
    }

    fn read_member(&mut self, text: &str) {
        if let Some(rest) = text.strip_prefix("attr_reader ") {
            Self::add_all(&mut self.fields, rest);
            return;
        }
        if let Some(rest) = text.strip_prefix("attr_accessor ") {
            Self::add_all(&mut self.fields, rest);
            Self::add_all(&mut self.writable, rest);
            return;
        }
        let Some(rest) = text.strip_prefix("def ") else {
            return;
        };
        let name = AsciiName::leading(rest);
        let plain_end = rest[name.len()..]
            .chars()
            .next()
            .is_none_or(|next| matches!(next, '(' | ' ' | ';'));
        let hidden = name.is_empty() || name == "initialize" || name.starts_with('_');
        if plain_end && !hidden && !self.methods.iter().any(|known| known == name) {
            self.methods.push(name.to_owned());
        }
    }

    fn add_all(names: &mut Vec<String>, symbols: &str) {
        for part in symbols.split(',') {
            let Some(name) = part.trim().strip_prefix(':') else {
                continue;
            };
            let valid = !name.is_empty()
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_');
            if valid && !names.iter().any(|known| known == name) {
                names.push(name.to_owned());
            }
        }
    }
}

pub(crate) struct Yard;

impl Yard {
    pub(crate) fn comments_before(source: &str, declaration: usize) -> String {
        let line_start = source[..declaration].rfind('\n').map_or(0, |at| at + 1);
        let mut lines = Vec::new();
        for line in source[..line_start].lines().rev() {
            let text = line.trim();
            let Some(comment) = text.strip_prefix('#') else {
                break;
            };
            if !comment.starts_with('[') {
                lines.push(comment);
            }
        }
        lines.reverse();
        lines.join("\n")
    }

    pub(crate) fn parameter_type(comments: &str, name: &str) -> String {
        for line in comments.lines() {
            let Some(rest) = line.trim().strip_prefix("@param") else {
                continue;
            };
            let rest = rest.trim_start();
            if let Some((types, after)) = Self::bracketed(rest)
                && Self::first_word(after) == name
            {
                return types.to_owned();
            }
            let written = Self::first_word(rest);
            if written != name {
                continue;
            }
            if let Some((types, _)) = Self::bracketed(rest[written.len()..].trim_start()) {
                return types.to_owned();
            }
        }
        String::new()
    }

    pub(crate) fn return_type(comments: &str) -> String {
        for line in comments.lines() {
            let Some(rest) = line.trim().strip_prefix("@return") else {
                continue;
            };
            if let Some((types, _)) = Self::bracketed(rest.trim_start()) {
                return types.to_owned();
            }
        }
        String::new()
    }

    pub(crate) fn attribute_types(comments: &str) -> Vec<(String, String)> {
        let mut attributes = Vec::new();
        let mut pending: Option<String> = None;
        for line in comments.lines() {
            let text = line.trim();
            if let Some(rest) = text.strip_prefix("@!attribute") {
                let rest = rest.trim_start();
                let rest = Self::bracketed(rest).map_or(rest, |(_, after)| after);
                pending = Some(Self::first_word(rest).to_owned());
                continue;
            }
            let Some(name) = pending.take() else {
                continue;
            };
            let Some(rest) = text.strip_prefix("@return") else {
                pending = Some(name);
                continue;
            };
            if let Some((types, _)) = Self::bracketed(rest.trim_start()) {
                attributes.push((name, types.to_owned()));
            }
        }
        attributes
    }

    fn bracketed(text: &str) -> Option<(&str, &str)> {
        let inside = text.strip_prefix('[')?;
        let mut depth = 0_usize;
        for (at, character) in inside.char_indices() {
            match character {
                '[' => depth += 1,
                ']' if depth == 0 => return Some((inside[..at].trim(), &inside[at + 1..])),
                ']' => depth -= 1,
                _ => {}
            }
        }
        None
    }

    fn first_word(text: &str) -> &str {
        AsciiName::leading(text.trim_start())
    }
}

pub(crate) struct RubyLexer<'a> {
    bytes: &'a [u8],
    source: &'a str,
    code: Vec<bool>,
    heredocs: Vec<Heredoc>,
    marker_problem: Option<(usize, &'static str)>,
}

struct Heredoc {
    label: String,
    indented: bool,
}

type LexicalError = (usize, &'static str);

impl<'a> RubyLexer<'a> {
    pub(crate) const SPACED_MARKER: &'static str = "Write Ruby annotations as `#[Name]`, with no space after `#`. RuboCop's Layout/LeadingCommentSpace rule adds that space; turn the rule off in .rubocop.yml";
    const EXPRESSION_KEYWORDS: [&'static str; 18] = [
        "if", "unless", "while", "until", "when", "case", "and", "or", "not", "return", "then",
        "else", "elsif", "in", "do", "puts", "print", "yield",
    ];
    const PERCENT_KINDS: &'static [u8] = b"qQwWiIrsx";
    const PERCENT_DELIMITERS: &'static [u8] = b"([{<|!/^";
    const CHARACTER_LITERALS: &'static [u8] = b"'\"`#/%<?\\";

    pub(crate) fn code_mask(source: &'a str) -> LexicalMask {
        let mut lexer = Self {
            bytes: source.as_bytes(),
            source,
            code: vec![true; source.len()],
            heredocs: Vec::new(),
            marker_problem: None,
        };
        let error = lexer.scan().err();
        let mut code = lexer.code;
        let error = error.map(|(start, message)| {
            code[start..].fill(false);
            message
        });
        LexicalMask {
            code,
            error,
            marker_problem: lexer.marker_problem,
        }
    }

    fn scan(&mut self) -> Result<(), LexicalError> {
        let mut at = 0;
        while at < self.bytes.len() {
            at = self.step(at)?;
        }
        if let Some(heredoc) = self.heredocs.first() {
            let start = self.source.rfind(&heredoc.label).unwrap_or(0);
            return Err((start, "Unterminated Ruby heredoc"));
        }
        Ok(())
    }

    fn step(&mut self, at: usize) -> Result<usize, LexicalError> {
        let byte = self.bytes[at];
        if byte == b'\n' {
            return self.after_newline(at);
        }
        if self.at_line_start(at) {
            if let Some(end) = self.block_comment(at)? {
                return Ok(self.hide(at, end));
            }
            if self.line_text(at) == "__END__" {
                return Ok(self.hide(at, self.bytes.len()));
            }
        }
        match byte {
            b'#' => Ok(self.comment(at)),
            b'"' | b'`' => {
                let end = self
                    .interpolated_end(at + 1, byte)
                    .ok_or((at, "Unterminated Ruby string"))?;
                Ok(self.hide(at, end))
            }
            b'\'' => {
                let end = self
                    .escaped_end(at + 1, b'\'')
                    .ok_or((at, "Unterminated Ruby string"))?;
                Ok(self.hide(at, end))
            }
            b'%' => self.percent_literal(at),
            b'<' => Ok(self.heredoc_start(at)),
            b'?' => Ok(self.character_literal(at)),
            b'/' => Ok(self.regular_expression(at)),
            b'$' => Ok(self.global_variable(at)),
            _ => Ok(at + 1),
        }
    }

    fn hide(&mut self, start: usize, end: usize) -> usize {
        let end = end.min(self.bytes.len());
        self.code[start..end].fill(false);
        end
    }

    fn at_line_start(&self, at: usize) -> bool {
        at == 0 || self.bytes[at - 1] == b'\n'
    }

    fn line_end(&self, at: usize) -> usize {
        self.source[at..]
            .find('\n')
            .map_or(self.bytes.len(), |offset| at + offset)
    }

    fn line_start(&self, at: usize) -> usize {
        self.source[..at].rfind('\n').map_or(0, |offset| offset + 1)
    }

    fn line_text(&self, at: usize) -> &str {
        self.source[at..self.line_end(at)].trim_end_matches('\r')
    }

    fn starts_word(line: &str, word: &str) -> bool {
        let Some(rest) = line.strip_prefix(word) else {
            return false;
        };
        rest.is_empty() || rest.starts_with([' ', '\t'])
    }

    fn block_comment(&self, at: usize) -> Result<Option<usize>, LexicalError> {
        if !Self::starts_word(self.line_text(at), "=begin") {
            return Ok(None);
        }
        let mut line = self.line_end(at) + 1;
        while line < self.bytes.len() {
            let end = self.line_end(line);
            if Self::starts_word(self.line_text(line), "=end") {
                return Ok(Some(end));
            }
            line = end + 1;
        }
        Err((at, "Unterminated Ruby =begin comment"))
    }

    fn comment(&mut self, at: usize) -> usize {
        let end = self.line_end(at);
        let first_on_line = self.source[self.line_start(at)..at].trim().is_empty();
        if first_on_line && self.bytes.get(at + 1) == Some(&b'[') {
            let text = &self.source[at + 2..end];
            if crate::go_source::GoMarkerName::is_marker(text) {
                return at + 1;
            }
        }
        if first_on_line {
            self.check_spaced_marker(at, end);
        }
        self.hide(at, end)
    }

    fn check_spaced_marker(&mut self, at: usize, end: usize) {
        let after_hash = &self.source[at + 1..end];
        let Some(rest) = after_hash.trim_start_matches([' ', '\t']).strip_prefix('[') else {
            return;
        };
        if after_hash.len() == rest.len() + 1 {
            return;
        }
        let closes_line = rest.trim_end().ends_with(']');
        if closes_line && crate::go_source::GoMarkerName::is_marker(rest) {
            self.marker_problem = self.marker_problem.or(Some((at, Self::SPACED_MARKER)));
        }
    }

    fn after_newline(&mut self, at: usize) -> Result<usize, LexicalError> {
        if self.heredocs.is_empty() {
            return Ok(at + 1);
        }
        let mut line = at + 1;
        for heredoc in std::mem::take(&mut self.heredocs) {
            let end = self
                .heredoc_end(line, &heredoc)
                .ok_or((at, "Unterminated Ruby heredoc"))?;
            self.hide(line, end);
            line = end + 1;
        }
        Ok(line.saturating_sub(1).max(at + 1))
    }

    fn heredoc_end(&self, mut line: usize, heredoc: &Heredoc) -> Option<usize> {
        while line < self.bytes.len() {
            let end = self.line_end(line);
            let text = self.line_text(line);
            let text = if heredoc.indented {
                text.trim_start()
            } else {
                text
            };
            if text == heredoc.label {
                return Some(end);
            }
            line = end + 1;
        }
        None
    }

    fn heredoc_start(&mut self, at: usize) -> usize {
        let repeated = at > 0 && self.bytes[at - 1] == b'<';
        if self.bytes.get(at + 1) != Some(&b'<') || repeated {
            return at + 1;
        }
        let mut next = at + 2;
        let indented = matches!(self.bytes.get(next), Some(b'~' | b'-'));
        if indented {
            next += 1;
        }
        let Some(&first) = self.bytes.get(next) else {
            return at + 1;
        };
        if matches!(first, b'\'' | b'"' | b'`') {
            let line_end = self.line_end(next);
            let Some(close) = self.source[next + 1..line_end].find(first as char) else {
                return at + 1;
            };
            let label = self.source[next + 1..next + 1 + close].to_owned();
            self.heredocs.push(Heredoc { label, indented });
            return next + close + 2;
        }
        let starts_label = if indented {
            first.is_ascii_alphabetic() || first == b'_'
        } else {
            first.is_ascii_uppercase() || first == b'_'
        };
        if !starts_label {
            return at + 1;
        }
        let mut end = next;
        while end < self.bytes.len() && Self::is_word_byte(self.bytes[end]) {
            end += 1;
        }
        let label = self.source[next..end].to_owned();
        self.heredocs.push(Heredoc { label, indented });
        end
    }

    fn percent_literal(&mut self, at: usize) -> Result<usize, LexicalError> {
        let Some(&first) = self.bytes.get(at + 1) else {
            return Ok(at + 1);
        };
        let delimiter_at = if Self::PERCENT_KINDS.contains(&first) {
            at + 2
        } else if Self::PERCENT_DELIMITERS.contains(&first) && self.expression_start(at) {
            at + 1
        } else {
            return Ok(at + 1);
        };
        let Some(&delimiter) = self.bytes.get(delimiter_at) else {
            return Ok(at + 1);
        };
        let is_delimiter = delimiter.is_ascii_punctuation() && delimiter != b'=';
        if !is_delimiter {
            return Ok(at + 1);
        }
        let end = self
            .delimited_end(delimiter_at + 1, delimiter)
            .ok_or((at, "Unterminated Ruby percent literal"))?;
        Ok(self.hide(at, end))
    }

    fn delimited_end(&self, mut at: usize, opening: u8) -> Option<usize> {
        let closing = match opening {
            b'(' => b')',
            b'[' => b']',
            b'{' => b'}',
            b'<' => b'>',
            other => other,
        };
        let nests = closing != opening;
        let mut depth = 0_usize;
        while at < self.bytes.len() {
            let byte = self.bytes[at];
            if byte == b'\\' {
                at += 2;
                continue;
            }
            if nests && byte == opening {
                depth += 1;
            } else if byte == closing && depth == 0 {
                return Some(at + 1);
            } else if byte == closing {
                depth -= 1;
            }
            at += 1;
        }
        None
    }

    fn character_literal(&mut self, at: usize) -> usize {
        let Some(&next) = self.bytes.get(at + 1) else {
            return at + 1;
        };
        if !Self::CHARACTER_LITERALS.contains(&next) || !self.expression_start(at) {
            return at + 1;
        }
        let length = if next == b'\\' { 3 } else { 2 };
        let after = self.bytes.get(at + length).copied();
        if after.is_some_and(Self::is_word_byte) {
            return at + 1;
        }
        self.hide(at, at + length)
    }

    fn regular_expression(&mut self, at: usize) -> usize {
        if !self.starts_regular_expression(at) {
            return at + 1;
        }
        let mut position = at + 1;
        let mut in_class = false;
        while position < self.bytes.len() {
            match self.bytes[position] {
                b'\\' => position += 2,
                b'\n' => return at + 1,
                b'[' => {
                    in_class = true;
                    position += 1;
                }
                b']' => {
                    in_class = false;
                    position += 1;
                }
                b'/' if !in_class => return self.hide(at, position + 1),
                _ => position += 1,
            }
        }
        at + 1
    }

    fn starts_regular_expression(&self, at: usize) -> bool {
        if self.expression_start(at) {
            return true;
        }
        let after = self.bytes.get(at + 1).copied();
        let spaced_before = at > 0 && matches!(self.bytes[at - 1], b' ' | b'\t');
        let word_before = self.previous_word(at).is_some();
        spaced_before
            && word_before
            && after.is_some_and(|byte| !matches!(byte, b' ' | b'\t' | b'='))
    }

    fn global_variable(&self, at: usize) -> usize {
        let next = self.bytes.get(at + 1).copied();
        if next.is_some_and(|byte| byte.is_ascii_punctuation()) {
            return at + 2;
        }
        at + 1
    }

    fn previous_word(&self, at: usize) -> Option<&str> {
        let before = self.source[..at].trim_end_matches([' ', '\t']);
        let start = before
            .bytes()
            .rposition(|byte| !Self::is_word_byte(byte))
            .map_or(0, |position| position + 1);
        let word = &before[start..];
        (!word.is_empty()).then_some(word)
    }

    fn expression_start(&self, at: usize) -> bool {
        let before = self.source[..at].trim_end_matches([' ', '\t']);
        let Some(&last) = before.as_bytes().last() else {
            return true;
        };
        if last == b'\n' {
            return true;
        }
        if !Self::is_word_byte(last) {
            return !matches!(last, b')' | b']' | b'}' | b'"' | b'\'' | b'`');
        }
        self.previous_word(at)
            .is_some_and(|word| Self::EXPRESSION_KEYWORDS.contains(&word))
    }

    fn is_word_byte(byte: u8) -> bool {
        byte.is_ascii_alphanumeric() || byte == b'_' || byte >= 0x80
    }

    fn escaped_end(&self, mut at: usize, quote: u8) -> Option<usize> {
        while at < self.bytes.len() {
            match self.bytes[at] {
                b'\\' => at += 2,
                byte if byte == quote => return Some(at + 1),
                _ => at += 1,
            }
        }
        None
    }

    fn interpolated_end(&self, mut at: usize, quote: u8) -> Option<usize> {
        while at < self.bytes.len() {
            match self.bytes[at] {
                b'\\' => at += 2,
                b'#' if self.bytes.get(at + 1) == Some(&b'{') => {
                    at = self.interpolation_end(at + 2)?;
                }
                byte if byte == quote => return Some(at + 1),
                _ => at += 1,
            }
        }
        None
    }

    fn interpolation_end(&self, mut at: usize) -> Option<usize> {
        let mut depth = 1_usize;
        while at < self.bytes.len() {
            match self.bytes[at] {
                b'{' => {
                    depth += 1;
                    at += 1;
                }
                b'}' => {
                    depth -= 1;
                    at += 1;
                    if depth == 0 {
                        return Some(at);
                    }
                }
                b'"' | b'`' => at = self.interpolated_end(at + 1, self.bytes[at])?,
                b'\'' => at = self.escaped_end(at + 1, b'\'')?,
                _ => at += 1,
            }
        }
        None
    }
}

#[cfg(test)]
mod member_tests {
    use super::{RubyLexer, RubyMembers};

    #[test]
    fn public_readers_accessors_and_methods_form_the_member_table() {
        let source = "class CounterService\n  attr_reader :count, :label\n  attr_accessor :step\n\n  def initialize\n    @count = 0\n  end\n\n  def increment(by = 1)\n    @count += by\n  end\n\n  def self.build\n  end\n\n  def empty?\n  end\n\n  private\n\n  def hidden\n  end\nend\n";
        let mask = RubyLexer::code_mask(source);
        let code: String = source
            .char_indices()
            .map(|(at, value)| if mask.code[at] { value } else { ' ' })
            .collect();
        let members = RubyMembers::read(&code);
        assert_eq!(members.fields, ["count", "label", "step"]);
        assert_eq!(members.writable, ["step"]);
        assert_eq!(members.methods, ["increment"]);
    }
}

#[cfg(test)]
mod yard_tests {
    use super::Yard;
    use crate::ascii_name::AsciiName;

    fn texts() -> Vec<String> {
        let alphabet = (0..=0x7f_u8)
            .map(char::from)
            .chain(['é', 'Ü', 'ß', '٣'])
            .collect::<Vec<_>>();
        let mut texts = vec![String::new()];
        for first in &alphabet {
            texts.push(first.to_string());
            texts.extend(alphabet.iter().map(|second| format!("{first}{second}")));
        }
        texts
    }

    fn leading_word(text: &str) -> &str {
        let length = text
            .bytes()
            .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
            .count();
        &text[..length]
    }

    #[test]
    fn yard_words_skip_leading_space_then_read_ascii_word_characters() {
        for text in texts() {
            for padded in [text.clone(), format!("  {text}"), format!("\t{text}")] {
                let expected = leading_word(padded.trim_start());
                assert_eq!(Yard::first_word(&padded), expected, "{padded:?}");
            }
            assert_eq!(AsciiName::leading(&text), leading_word(&text), "{text:?}");
        }
    }

    #[test]
    fn parameter_and_return_types_come_from_yard_tags() {
        let source = "class A\n  # Finds an order.\n  # @param id [Integer] the order number\n  # @param [String, nil] note\n  #[GetRoute(\"/{id}\")]\n  # @return [Array<Order>] the orders\n  def get_order(id, note)\n";
        let declaration = source.find("def").unwrap_or_default();
        let comments = Yard::comments_before(source, declaration);
        assert_eq!(Yard::parameter_type(&comments, "id"), "Integer");
        assert_eq!(Yard::parameter_type(&comments, "note"), "String, nil");
        assert_eq!(Yard::parameter_type(&comments, "other"), "");
        assert_eq!(Yard::return_type(&comments), "Array<Order>");
        assert!(!comments.contains("GetRoute"), "{comments}");
    }

    #[test]
    fn attribute_directives_pair_names_with_types() {
        let comments = " @!attribute [r] id\n   @return [String]\n @!attribute [r] lines\n   @return [Array<Line>]";
        assert_eq!(
            Yard::attribute_types(comments),
            [
                (String::from("id"), String::from("String")),
                (String::from("lines"), String::from("Array<Line>")),
            ]
        );
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod marker_tests {
    use super::RubyMarkers;
    use std::path::Path;

    fn problem(source: &str) -> Option<String> {
        let code =
            crate::annotation_layout::masked_source(Path::new("server/services/A.rb"), source)
                .expect("masked");
        RubyMarkers::validate(Path::new("server/services/A.rb"), source, &code)
            .err()
            .map(|failure| failure.to_string())
    }

    #[test]
    fn known_markers_and_ordinary_comments_pass() {
        let source = "#[Service]\nclass AService\n  # [x] a plain note\n  #[deprecated]\n  def a; end\nend\n";
        assert_eq!(problem(source), None);
    }

    #[test]
    fn near_misses_and_bare_markers_are_refused() {
        let typo = problem("#[Servce]\nclass AService\nend\n").expect("typo");
        assert!(typo.contains("did you mean `#[Service]`"), "{typo}");
        let bare = problem("@Service\nclass AService\nend\n").expect("bare");
        assert!(bare.contains("write `#[Service]`"), "{bare}");
        let spaced = problem("# [Service]\nclass AService\nend\n").expect("spaced");
        assert!(spaced.contains("RuboCop"), "{spaced}");
        let namespace = problem("#[Yon::Service]\nclass AService\nend\n").expect("namespace");
        assert!(namespace.contains("#[yon::Service]"), "{namespace}");
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::RubyLexer;

    fn visible_code(source: &str) -> String {
        let mask = RubyLexer::code_mask(source);
        assert!(mask.error.is_none(), "{source}: {:?}", mask.error);
        source
            .char_indices()
            .map(|(at, value)| if mask.code[at] { value } else { ' ' })
            .collect()
    }

    #[test]
    fn comments_are_hidden_but_marker_lines_stay_visible() {
        let source = "# don't \"stop\"\n#[Service]\nclass Store # it's here\n  #[GetRoute(\"/{id}\")]\n  def get_item; end\nend\n";
        let masked = visible_code(source);
        assert!(masked.contains("#[Service]"), "{masked}");
        assert!(masked.contains("#[GetRoute("), "{masked}");
        assert!(
            !masked.contains("don't") && !masked.contains("it's"),
            "{masked}"
        );
        assert!(!masked.contains("/{id}"), "{masked}");
        assert!(masked.contains("class Store") && masked.contains("def get_item"));
    }

    #[test]
    fn ordinary_bracket_comments_and_inline_markers_stay_comments() {
        let masked = visible_code("#[deprecated]\nx = 1 #[Service]\n");
        assert!(!masked.contains("deprecated"), "{masked}");
        assert!(!masked.contains("Service"), "{masked}");
    }

    #[test]
    fn heredocs_and_block_comments_hide_marker_lookalikes() {
        let source = "text = <<~TEXT\n  #[Service]\n  it's\nTEXT\n=begin\n#[Service]\n=end\nsql = <<-'SQL' + <<EOS\nx\n  SQL\n#[Store]\nEOS\n#[Helper]\nclass A; end\n";
        let masked = visible_code(source);
        assert!(
            !masked.contains("Service") && !masked.contains("Store"),
            "{masked}"
        );
        assert!(
            masked.contains("#[Helper]") && masked.contains("class A"),
            "{masked}"
        );
        assert!(masked.contains("text = <<~TEXT"), "{masked}");
    }

    #[test]
    fn interpolation_percent_literals_and_regular_expressions_are_hidden() {
        let source = "a = \"x #{h[\"}\"] + '{'} y\"; GET()\nb = %w[it's #{x}] ; c = %q(a (b) ') ; POST()\nd = text.split /'/ ; e = 4 / 2 ; f = ?' ; PUT()\ng = $' ; h = x % 3 ; DELETE()\n";
        let masked = visible_code(source);
        for call in ["GET()", "POST()", "PUT()", "DELETE()"] {
            assert!(masked.contains(call), "{call}: {masked}");
        }
        assert!(
            masked.contains("4 / 2") && masked.contains("x % 3"),
            "{masked}"
        );
        assert!(
            !masked.contains("it's") && !masked.contains('}'),
            "{masked}"
        );
    }

    #[test]
    fn a_spaced_marker_is_reported_as_a_rubocop_change() {
        let mask = RubyLexer::code_mask("# [Service]\nclass Store\nend\n# [Order] is fine too\n");
        assert_eq!(mask.marker_problem.map(|(at, _)| at), Some(0));
        let plain = RubyLexer::code_mask("# [Order] the first order\n# @return [String]\n");
        assert!(plain.marker_problem.is_none());
    }

    #[test]
    fn unterminated_literals_are_errors() {
        for source in [
            "x = \"open\n",
            "=begin\nnever closed\n",
            "x = <<~EOS\nbody\n",
            "x = %w[a b\n",
        ] {
            assert!(RubyLexer::code_mask(source).error.is_some(), "{source}");
        }
    }

    #[test]
    fn data_after_end_marker_is_hidden() {
        let masked = visible_code("class A\nend\n__END__\n#[Service] it's data\n");
        assert!(!masked.contains("Service"), "{masked}");
    }
}
