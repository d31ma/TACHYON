use super::super::tokens::Token;
use super::import_words::ImportWords;
use super::project_source::ProjectSource;
use super::rust_declarations::RustDeclarations;
use super::{Input, Tokens};
use std::ops::Range;
use std::path::Path;
use tachyon_duplication::{CodeToken, SourceFile, TokenKind};

pub(super) struct CodeReader<'r, 'a> {
    tokens: &'r Tokens<'a>,
    imports: &'static [&'static str],
    depths: Vec<usize>,
    line_starts: Vec<usize>,
    declarations: Vec<Range<usize>>,
}

impl<'r, 'a> CodeReader<'r, 'a> {
    const OPENERS: [&'static str; 3] = ["(", "[", "{"];
    const CLOSERS: [&'static str; 3] = [")", "]", "}"];

    pub(super) fn read(source: &ProjectSource) -> Option<SourceFile> {
        let input = Input::new(Path::new(&source.path), &source.text);
        let tokens = Tokens::read(&input).ok()?;
        let reader = CodeReader::new(&tokens, &source.text, source.extension());
        let kept = reader.kept();
        let production = !tokens.has_test_marker();
        production.then(|| SourceFile::new(source.path.clone(), source.language(), kept))
    }

    fn new(tokens: &'r Tokens<'a>, text: &str, extension: &str) -> Self {
        Self {
            tokens,
            imports: ImportWords::of(extension),
            depths: Self::depths(tokens),
            line_starts: Self::line_starts(text),
            declarations: RustDeclarations::read(tokens, text, extension),
        }
    }

    fn depths(tokens: &Tokens<'_>) -> Vec<usize> {
        let mut depth = 0_usize;
        let mut depths = Vec::with_capacity(tokens.items.len());
        for token in &tokens.items {
            depths.push(depth);
            depth = Self::depth_after(depth, token);
        }
        depths
    }

    fn depth_after(depth: usize, token: &Token<'_>) -> usize {
        let code = !token.literal;
        let opens = usize::from(code & Self::OPENERS.contains(&token.text));
        let closes = usize::from(code & Self::CLOSERS.contains(&token.text));
        (depth + opens).saturating_sub(closes)
    }

    fn line_starts(text: &str) -> Vec<usize> {
        std::iter::once(0)
            .chain(text.match_indices('\n').map(|(at, _)| at + 1))
            .collect()
    }

    fn kept(&self) -> Vec<CodeToken> {
        let mut kept = Vec::new();
        let mut at = 0;
        while at < self.len() {
            let end = self.skipped_end(at);
            kept.extend((end == at).then(|| self.code_token(at)));
            at = end.max(at + 1);
        }
        kept
    }

    fn len(&self) -> usize {
        self.tokens.items.len()
    }

    fn text(&self, at: usize) -> &str {
        self.tokens.items.get(at).map_or("", |token| token.text)
    }

    fn skipped_end(&self, at: usize) -> usize {
        self.import_end(at)
            .or_else(|| self.tokens.marker_end(at))
            .or_else(|| self.declaration_end(at))
            .unwrap_or(at)
    }

    fn declaration_end(&self, at: usize) -> Option<usize> {
        self.declarations
            .iter()
            .find(|range| range.contains(&at))
            .map(|range| range.end)
    }

    fn import_end(&self, at: usize) -> Option<usize> {
        let first_on_line = self.tokens.new_line_before(at);
        let import_word = self.imports.contains(&self.text(at));
        let not_a_call = self.text(at + 1) != "(";
        (first_on_line & import_word & not_a_call).then(|| self.statement_end(at))
    }

    fn statement_end(&self, at: usize) -> usize {
        let depth = self.depths[at];
        (at + 1..self.len())
            .find(|next| self.tokens.new_line_before(*next) & (self.depths[*next] <= depth))
            .unwrap_or(self.len())
    }

    fn code_token(&self, at: usize) -> CodeToken {
        let token = &self.tokens.items[at];
        CodeToken::new(token.text, self.line(token.start), Self::kind(token))
    }

    fn line(&self, start: usize) -> usize {
        self.line_starts
            .partition_point(|line_start| *line_start <= start)
    }

    fn kind(token: &Token<'_>) -> TokenKind {
        let number = token.text.starts_with(|first: char| first.is_ascii_digit());
        match (token.literal | number, token.is_name()) {
            (true, _) => TokenKind::Literal,
            (false, true) => TokenKind::Name,
            (false, false) => TokenKind::Symbol,
        }
    }
}
