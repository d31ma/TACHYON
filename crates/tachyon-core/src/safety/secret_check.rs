use super::{SafetyUtils, SecretAssignment, SecretMatch, SecretPattern};
use std::path::Path;
use tachyon_diagnostics::Diagnostic;

pub(crate) struct SecretCheck;

impl SecretCheck {
    const CODE: u16 = 1324;
    const MARKER: &'static str = "tachyon: example secrets";
    const EXAMPLE_WORDS: [&'static str; 7] = [
        "example",
        "sample",
        "fake",
        "dummy",
        "placeholder",
        "changeme",
        "notreal",
    ];
    const TEXT_EXTENSIONS: &'static str =
        "js mjs cjs ts py php java kt cs rs dart go rb swift html css json yaml yml toml";
    const HELP: &'static str = "Secrets never go in source code, templates or public configuration. Put the value in the private file .env with a YON_ name and read it through the Yon configuration (docs/APPLICATION_CONFIGURATION.md, \"Which secrets are allowed\"); code that runs in the browser gets no secrets at all. A test or a guide may keep an obvious example value: write a comment with `tachyon: example secrets` in the file and make the value say that it is an example, such as `sk_live_example`.";

    pub(crate) fn diagnostics<'a>(
        files: impl Iterator<Item = (&'a Path, &'a [u8])>,
    ) -> Vec<Diagnostic> {
        SafetyUtils::each_file(files, Self::file_diagnostics)
    }

    fn file_diagnostics(path: &Path, bytes: &[u8]) -> Vec<Diagnostic> {
        let text = Self::text(path, bytes);
        text.map_or_else(Vec::new, |text| Self::text_diagnostics(path, text))
    }

    fn text<'a>(path: &Path, bytes: &'a [u8]) -> Option<&'a str> {
        Self::is_text_file(path)
            .then(|| std::str::from_utf8(bytes).ok())
            .flatten()
    }

    fn is_text_file(path: &Path) -> bool {
        path.extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                Self::TEXT_EXTENSIONS
                    .split(' ')
                    .any(|known| known == extension)
            })
    }

    fn text_diagnostics(path: &Path, text: &str) -> Vec<Diagnostic> {
        let refused = Self::refused(Self::matches(text), text.contains(Self::MARKER));
        refused
            .iter()
            .map(|found| Self::diagnostic(path, found))
            .collect()
    }

    fn matches(text: &str) -> Vec<SecretMatch> {
        text.lines()
            .enumerate()
            .flat_map(|(index, line)| Self::line_matches(line, index + 1))
            .collect()
    }

    fn line_matches(line: &str, number: usize) -> Vec<SecretMatch> {
        let found = SecretPattern::find(line, number);
        if found.is_empty() {
            return SecretAssignment::find(line, number).into_iter().collect();
        }
        found
    }

    fn refused(found: Vec<SecretMatch>, marked: bool) -> Vec<SecretMatch> {
        found
            .into_iter()
            .filter(|found| !Self::allowed(found, marked))
            .collect()
    }

    fn allowed(found: &SecretMatch, marked: bool) -> bool {
        marked && Self::looks_like_example(&found.value)
    }

    fn looks_like_example(value: &str) -> bool {
        let plain = value.replace(['-', '_'], "").to_ascii_lowercase();
        Self::EXAMPLE_WORDS.iter().any(|word| plain.contains(word))
    }

    fn diagnostic(path: &Path, found: &SecretMatch) -> Diagnostic {
        let portable = SafetyUtils::portable(path);
        let message = format!(
            "'{portable}' line {} holds what looks like {}. A secret in the source reaches everyone who can read the code or the bundle.",
            found.line, found.kind
        );
        SafetyUtils::refusal(Self::CODE, message, Self::HELP, (&portable, 0))
    }
}
