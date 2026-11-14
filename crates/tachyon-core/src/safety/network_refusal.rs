use super::SafetyUtils;
use crate::failure::{diagnostic, source_span};
use std::ops::Range;
use std::path::Path;
use tachyon_diagnostics::Diagnostic;

pub(crate) struct NetworkRefusal;

impl NetworkRefusal {
    const CODE: u16 = 1361;

    pub(crate) fn diagnostic(path: &Path, source: &str, used: Range<usize>) -> Diagnostic {
        let portable = SafetyUtils::portable(path);
        let class = crate::class_name_from_file::ClassNameFromFile::class_of(path);
        let call = source.get(used.clone()).unwrap_or_default();
        let line = SafetyUtils::line_at(source, used.start);
        diagnostic(
            Self::CODE,
            Self::message(&class, call, &portable, line),
            Some(Self::help(&class, call)),
            source_span(&portable, used.start, used.end),
        )
    }

    fn message(class: &str, call: &str, portable: &str, line: usize) -> String {
        format!(
            "{class} calls the network itself with `{call}` in '{portable}' line {line}. Tac code reaches the network only through a remote: an HTTPClient in client/remotes."
        )
    }

    fn help(class: &str, call: &str) -> String {
        format!(
            "Remove `{call}`. Move the request into a method of a remote in client/remotes, such as `@GetRequest(\"/api/products\")`, inject that remote into {class} with `@Needs`, and call its method. Mark a public read `Cached`, so the service worker may answer it (ADR 0087)."
        )
    }
}
