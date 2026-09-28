use crate::Failure;
use crate::failure::diagnostic;
use sha2::{Digest, Sha256};
use std::path::Path;

pub(crate) struct Sha256Check;

impl Sha256Check {
    const CODE: u16 = 1409;

    pub(crate) fn published(file: &Path, list: &Path) -> Result<(), Failure> {
        let text = Self::read_text(list)?;
        let expected = Self::listed_hash(&text, &Self::file_name(file));
        Self::compare(file, &expected)
    }

    pub(crate) fn pinned(file: &Path, expected: &str) -> Result<(), Failure> {
        Self::compare(file, expected)
    }

    pub(crate) fn listed_hash(text: &str, name: &str) -> String {
        let line = Self::line_for(text, name).unwrap_or_default();
        let hash = line.split_whitespace().next().unwrap_or_default();
        hash.to_ascii_lowercase()
    }

    fn line_for<'text>(text: &'text str, name: &str) -> Option<&'text str> {
        text.lines()
            .find(|line| Self::names_file(line, name))
            .or_else(|| text.lines().next())
    }

    fn names_file(line: &str, name: &str) -> bool {
        let listed = line.split_whitespace().last().unwrap_or_default();
        listed.trim_start_matches('*').trim_start_matches("./") == name
    }

    fn compare(file: &Path, expected: &str) -> Result<(), Failure> {
        let bytes = std::fs::read(file).map_err(|error| Self::failure(file, &error.to_string()))?;
        let actual = crate::compiler::hex_digest(Sha256::digest(&bytes));
        Self::matches(file, &actual, expected)
    }

    fn matches(file: &Path, actual: &str, expected: &str) -> Result<(), Failure> {
        if actual == expected {
            return Ok(());
        }
        let _ = std::fs::remove_file(file);
        Err(Self::failure(
            file,
            &format!(
                "its SHA-256 is {actual}, but the vendor says {expected}. Tachyon deleted the download and stopped"
            ),
        ))
    }

    fn read_text(path: &Path) -> Result<String, Failure> {
        std::fs::read_to_string(path).map_err(|error| Self::failure(path, &error.to_string()))
    }

    fn file_name(file: &Path) -> String {
        file.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    fn failure(file: &Path, reason: &str) -> Failure {
        Failure::one(diagnostic(
            Self::CODE,
            format!(
                "The download {} cannot be trusted: {reason}.",
                file.display()
            ),
            Some(String::from(
                "Try again later. If it fails again, install the tool yourself from its official website.",
            )),
            None,
        ))
    }
}
