use super::{DelegateBase, DelegateForwards};
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use std::path::Path;

pub(super) struct TacBase;

impl TacBase {
    const CODE: u16 = 1406;
    const TAC_REALM: &'static str = "client";
    const UNSHARED: [(&'static str, &'static str); 5] = [
        ("kt", "Kotlin"),
        ("swift", "Swift"),
        ("cs", "C#"),
        ("rb", "Ruby"),
        ("lua", "Lua"),
    ];
    const HELP: &'static str = "Let the view call the service, or keep this Delegate without a service field: a Delegate without a base works in every language. A Delegate can take on a service in the Tac languages whose services share one instance: JavaScript, TypeScript, Python, Dart, Go, Java, C++ and F# (ADR 0112).";

    pub(super) fn check(path: &Path, source: &str, base: &DelegateBase<'_>) -> Result<(), Failure> {
        let language = path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default();
        Self::UNSHARED
            .iter()
            .filter(|_| path.starts_with(Self::TAC_REALM))
            .find(|(extension, _)| *extension == language)
            .map_or(Ok(()), |(_, name)| {
                Err(Self::refused(path, source, base, name))
            })
    }

    fn refused(path: &Path, source: &str, base: &DelegateBase<'_>, language: &str) -> Failure {
        let portable = path.to_string_lossy().replace('\\', "/");
        let service = &base.service;
        let message = format!(
            "{} takes on {service}, but a {language} browser service cannot need another service, so the page has no {service} to give it (ADR 0065).",
            DelegateForwards::class_name(path),
        );
        let end = (base.field_at + base.field.len()).min(source.len());
        Failure::one(diagnostic(
            Self::CODE,
            message,
            Some(Self::HELP.to_owned()),
            source_span(&portable, base.field_at, end),
        ))
    }
}
