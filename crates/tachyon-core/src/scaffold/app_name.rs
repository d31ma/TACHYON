use super::ScaffoldFailure;
use crate::Failure;
use std::path::Path;

pub(crate) struct AppName;

impl AppName {
    const FALLBACK: &'static str = "Tachyon App";
    const FALLBACK_PACKAGE: &'static str = "tachyon-app";
    const LONGEST: usize = 100;
    const PACKAGE_PUNCTUATION: [char; 3] = ['.', '_', '-'];

    pub(crate) fn validated(provided: Option<&str>, target: &Path) -> Result<String, Failure> {
        let fallback = Self::folder_name(target);
        let name = provided.unwrap_or(&fallback).trim();
        Self::printable(name)
    }

    pub(crate) fn package(app_name: &str) -> String {
        let lower = app_name.trim().to_lowercase();
        let words: Vec<&str> = lower
            .split(Self::is_separator)
            .filter(|word| !word.is_empty())
            .collect();
        let joined = words.join("-");
        Self::package_or_fallback(joined.trim_matches('-'))
    }

    pub(crate) fn javascript_literal(app_name: &str) -> String {
        serde_json::to_string(app_name).unwrap_or_else(|_| String::from("\"Tachyon App\""))
    }

    fn folder_name(target: &Path) -> String {
        let name = target
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        Self::name_or_fallback(name)
    }

    fn name_or_fallback(name: &str) -> String {
        if name.is_empty() {
            return String::from(Self::FALLBACK);
        }
        String::from(name)
    }

    fn printable(name: &str) -> Result<String, Failure> {
        let problems = [
            name.is_empty(),
            name.chars().count() > Self::LONGEST,
            name.chars().any(char::is_control),
        ];
        if problems.contains(&true) {
            return Err(ScaffoldFailure::name());
        }
        Ok(String::from(name))
    }

    fn is_separator(character: char) -> bool {
        let allowed = [
            character.is_ascii_alphanumeric(),
            Self::PACKAGE_PUNCTUATION.contains(&character),
        ];
        !allowed.contains(&true)
    }

    fn package_or_fallback(package: &str) -> String {
        if package.is_empty() {
            return String::from(Self::FALLBACK_PACKAGE);
        }
        String::from(package)
    }
}
