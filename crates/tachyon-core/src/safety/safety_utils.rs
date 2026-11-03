use crate::failure::{diagnostic, source_span};
use std::path::Path;
use tachyon_diagnostics::Diagnostic;

pub(super) struct SafetyUtils;

impl SafetyUtils {
    pub(super) fn each_file<'a>(
        files: impl Iterator<Item = (&'a Path, &'a [u8])>,
        check: impl Fn(&Path, &[u8]) -> Vec<Diagnostic>,
    ) -> Vec<Diagnostic> {
        files.flat_map(|(path, bytes)| check(path, bytes)).collect()
    }

    pub(super) fn portable(path: &Path) -> String {
        path.to_string_lossy().replace('\\', "/")
    }

    pub(super) fn line_at(text: &str, at: usize) -> usize {
        text.get(..at).unwrap_or(text).matches('\n').count() + 1
    }

    pub(super) fn refusal(
        code: u16,
        message: String,
        help: &'static str,
        place: (&str, usize),
    ) -> Diagnostic {
        let (portable, at) = place;
        diagnostic(
            code,
            message,
            Some(String::from(help)),
            source_span(portable, at, at + 1),
        )
    }
}
