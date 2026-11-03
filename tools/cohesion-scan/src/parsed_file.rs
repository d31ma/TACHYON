use crate::ScanError;
use crate::rust_tokens::RustTokens;
use crate::source_files::SourceFiles;
use std::fs;
use std::path::Path;
use syn::File;
use tachyon_duplication::{Duplicate, DuplicateFinder, SourceFile};

pub(crate) struct ParsedFile {
    pub(crate) relative: String,
    pub(crate) syntax: File,
}

impl ParsedFile {
    pub(crate) fn read(root: &Path, path: &Path) -> Result<Self, ScanError> {
        let source = fs::read_to_string(path).map_err(|error| ScanError::new(path, &error))?;
        let syntax = syn::parse_file(&source).map_err(|error| ScanError::new(path, &error))?;
        Ok(Self {
            relative: SourceFiles::relative(root, path),
            syntax,
        })
    }

    pub(crate) fn duplicates(files: &[Self]) -> Vec<String> {
        let sources: Vec<SourceFile> = files.iter().map(Self::tokens).collect();
        DuplicateFinder::find(&sources)
            .iter()
            .map(Self::describe)
            .collect()
    }

    fn tokens(&self) -> SourceFile {
        RustTokens::source_file(&self.relative, &self.syntax)
    }

    fn describe(duplicate: &Duplicate) -> String {
        format!(
            "duplicate  {} repeats {} ({} tokens)",
            duplicate.second.label(),
            duplicate.first.label(),
            duplicate.tokens
        )
    }
}
