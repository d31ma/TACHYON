use crate::source_file::SourceFile;
use crate::token_kind::TokenKind;
use std::collections::HashMap;

pub(crate) struct TokenStream<'f> {
    files: Vec<&'f SourceFile>,
    pub(crate) ids: Vec<u64>,
    lines: Vec<usize>,
    owners: Vec<usize>,
    words: Vec<bool>,
    names: HashMap<String, u64>,
}

impl<'f> TokenStream<'f> {
    pub(crate) fn new(files: Vec<&'f SourceFile>) -> Self {
        let mut stream = Self {
            files,
            ids: Vec::new(),
            lines: Vec::new(),
            owners: Vec::new(),
            words: Vec::new(),
            names: HashMap::new(),
        };
        stream.add_files();
        stream
    }

    pub(crate) fn line(&self, at: usize) -> usize {
        self.lines.get(at).copied().unwrap_or(0)
    }

    pub(crate) fn words_in(&self, start: usize, length: usize) -> usize {
        self.words
            .iter()
            .skip(start)
            .take(length)
            .filter(|word| **word)
            .count()
    }

    pub(crate) fn path(&self, at: usize) -> &str {
        let owner = self.owners.get(at).copied().unwrap_or(0);
        self.files.get(owner).map_or("", |file| file.path.as_str())
    }

    fn add_files(&mut self) {
        for index in 0..self.files.len() {
            self.add_file(index);
        }
    }

    fn add_file(&mut self, index: usize) {
        let file = self.files[index];
        for token in &file.tokens {
            let id = self.id_of(token.normalized());
            self.push(id, token.line, index, token.kind == TokenKind::Name);
        }
        self.push(Self::end_of_file(index), 0, index, false);
    }

    fn id_of(&mut self, key: &str) -> u64 {
        let next = self.names.len() as u64;
        *self.names.entry(key.to_owned()).or_insert(next)
    }

    fn end_of_file(index: usize) -> u64 {
        u64::MAX - index as u64
    }

    fn push(&mut self, id: u64, line: usize, owner: usize, word: bool) {
        self.ids.push(id);
        self.lines.push(line);
        self.owners.push(owner);
        self.words.push(word);
    }
}
