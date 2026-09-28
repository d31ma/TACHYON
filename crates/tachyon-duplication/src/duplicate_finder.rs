use crate::duplicate::Duplicate;
use crate::language_scan::LanguageScan;
use crate::source_file::SourceFile;
use crate::token_stream::TokenStream;
use std::collections::BTreeMap;

pub struct DuplicateFinder;

impl DuplicateFinder {
    pub const MIN_TOKENS: usize = 100;
    pub const MIN_LINES: usize = 10;

    #[must_use]
    pub fn find(files: &[SourceFile]) -> Vec<Duplicate> {
        let mut duplicates: Vec<Duplicate> = Self::by_language(files)
            .into_values()
            .flat_map(Self::in_language)
            .collect();
        duplicates.sort();
        duplicates
    }

    fn by_language(files: &[SourceFile]) -> BTreeMap<&str, Vec<&SourceFile>> {
        let mut languages: BTreeMap<&str, Vec<&SourceFile>> = BTreeMap::new();
        for file in files {
            languages
                .entry(file.language.as_str())
                .or_default()
                .push(file);
        }
        languages
    }

    fn in_language(files: Vec<&SourceFile>) -> Vec<Duplicate> {
        let stream = TokenStream::new(files);
        LanguageScan::new(&stream).duplicates()
    }
}
