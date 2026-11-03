use super::BrowserInteropCheck;
use super::interop_finder::InteropFinder;
use crate::failure::{diagnostic, source_span};
use std::ops::Range;
use std::path::Path;
use tachyon_diagnostics::Diagnostic;

pub(super) struct BrowserProvider {
    type_name: &'static str,
    browser_languages: &'static [&'static str],
    help: &'static str,
}

impl BrowserProvider {
    const SOURCE_LANGUAGES: [&'static str; 13] = [
        "java", "js", "ts", "rs", "cs", "swift", "kt", "dart", "go", "py", "rb", "lua", "fs",
    ];
    pub(super) const fn new(
        type_name: &'static str,
        browser_languages: &'static [&'static str],
        help: &'static str,
    ) -> Self {
        Self {
            type_name,
            browser_languages,
            help,
        }
    }

    pub(super) fn browser(&self, path: &Path, source: &str) -> Vec<Diagnostic> {
        let extension = Self::extension(path);
        self.browser_problems(
            [
                path.starts_with("client"),
                self.browser_languages.contains(&extension),
                Self::SOURCE_LANGUAGES.contains(&extension),
            ],
            path,
            source,
        )
    }

    pub(super) fn native(&self, path: &Path, source: &str) -> Vec<Diagnostic> {
        self.problems(path, source, "linked native platform")
    }

    fn browser_problems(&self, selection: [bool; 3], path: &Path, source: &str) -> Vec<Diagnostic> {
        match selection {
            [true, false, true] => self.problems(path, source, "browser language"),
            _ => Vec::new(),
        }
    }

    fn problems(&self, path: &Path, source: &str, platform: &str) -> Vec<Diagnostic> {
        InteropFinder::new(Self::extension(path), source)
            .all_words(&[self.type_name])
            .into_iter()
            .map(|range| self.problem(path, platform, range))
            .collect()
    }

    fn extension(path: &Path) -> &str {
        path.extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default()
    }

    fn problem(&self, path: &Path, platform: &str, range: Range<usize>) -> Diagnostic {
        let file = path.to_string_lossy().replace('\\', "/");
        diagnostic(
            BrowserInteropCheck::CODE,
            format!(
                "{} has no supported provider for this {platform}.",
                self.type_name
            ),
            Some(self.help.to_owned()),
            source_span(&file, range.start, range.end),
        )
    }
}
