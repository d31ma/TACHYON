use super::BrowserInteropCheck;
use super::interop_finder::InteropFinder;
use crate::failure::{diagnostic, source_span};
use std::ops::Range;
use std::path::Path;
use tachyon_diagnostics::Diagnostic;

pub(super) struct PreferenceProviderCheck;

impl PreferenceProviderCheck {
    const BROWSER_LANGUAGES: [&'static str; 3] = ["java", "js", "ts"];
    const SOURCE_LANGUAGES: [&'static str; 13] = [
        "java", "js", "ts", "rs", "cs", "swift", "kt", "dart", "go", "py", "rb", "lua", "fs",
    ];
    const TYPE: &'static str = "PreferenceStrings";
    const HELP: &'static str = "PreferenceStrings currently has Java browser and JavaScript/TypeScript page providers. Keep raw-string browser preferences in those services and pass results through the existing typed event contract. A linked native or another language provider requires its own adapter.";

    pub(super) fn browser(path: &Path, source: &str) -> Vec<Diagnostic> {
        let client = path.starts_with("client");
        let supported = Self::BROWSER_LANGUAGES.contains(&Self::extension(path));
        let source_language = Self::SOURCE_LANGUAGES.contains(&Self::extension(path));
        Self::browser_problems([client, supported, source_language], path, source)
    }

    pub(super) fn native(path: &Path, source: &str) -> Vec<Diagnostic> {
        Self::problems(path, source, "linked native platform")
    }

    fn browser_problems(selection: [bool; 3], path: &Path, source: &str) -> Vec<Diagnostic> {
        match selection {
            [true, false, true] => Self::problems(path, source, "browser language"),
            _ => Vec::new(),
        }
    }

    fn problems(path: &Path, source: &str, platform: &str) -> Vec<Diagnostic> {
        InteropFinder::new(Self::extension(path), source)
            .all_words(&[Self::TYPE])
            .into_iter()
            .map(|range| Self::problem(path, platform, range))
            .collect()
    }

    fn extension(path: &Path) -> &str {
        path.extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default()
    }

    fn problem(path: &Path, platform: &str, range: Range<usize>) -> Diagnostic {
        let file = path.to_string_lossy().replace('\\', "/");
        diagnostic(
            BrowserInteropCheck::CODE,
            format!("PreferenceStrings has no supported provider for this {platform}."),
            Some(Self::HELP.to_owned()),
            source_span(&file, range.start, range.end),
        )
    }
}
