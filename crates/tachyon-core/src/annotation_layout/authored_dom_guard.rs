use crate::Failure;
use crate::failure::{diagnostic, source_span};
use dom_scan::DomScan;
use std::path::Path;

mod dom_bindings;
mod dom_expression;
mod dom_kind;
mod dom_scan;
mod dom_statement;

#[cfg(test)]
mod tests;

pub(crate) struct AuthoredDomGuard;

impl AuthoredDomGuard {
    const CODE: u16 = 1364;
    const EXTENSIONS: [&'static str; 3] = ["js", "ts", "mjs"];
    const HELP: &'static str = "Put markup in client/components/<component>/tac.html and styles in that component's tac.css. Update text, classes and Service state from JavaScript or TypeScript. Load pinned vendor resources through tac.scripts and verified tac.scriptSources.";

    pub(crate) fn check(path: &Path, source: &str) -> Result<(), Failure> {
        Self::eligible(path)
            .then_some(path)
            .map_or(Ok(()), |path| Self::check_source(path, source))
    }

    pub(crate) fn check_bytes(path: &Path, bytes: &[u8]) -> Result<(), Failure> {
        Self::eligible(path)
            .then_some(path)
            .map_or(Ok(()), |path| Self::check_text(path, bytes))
    }

    fn eligible(path: &Path) -> bool {
        [
            path.starts_with("client"),
            path.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| Self::EXTENSIONS.contains(&extension)),
        ]
        .into_iter()
        .all(|holds| holds)
    }

    fn check_text(path: &Path, bytes: &[u8]) -> Result<(), Failure> {
        let source = std::str::from_utf8(bytes)
            .map_err(|_| Self::failure(path, 0, 0, "Client script text must be UTF-8"))?;
        Self::check_source(path, source)
    }

    fn check_source(path: &Path, source: &str) -> Result<(), Failure> {
        let mut scan = DomScan::new(path, source)?;
        scan.first_sink().map_or(Ok(()), |range| {
            Err(Self::failure(
                path,
                range.start,
                range.end,
                "Authored Tac scripts cannot construct component markup or write component styles",
            ))
        })
    }

    fn failure(path: &Path, start: usize, end: usize, message: &str) -> Failure {
        let portable = path.to_string_lossy().replace('\\', "/");
        Failure::one(diagnostic(
            Self::CODE,
            message,
            Some(Self::HELP.to_owned()),
            source_span(&portable, start, end),
        ))
    }
}
