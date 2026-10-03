use super::BrowserInteropCheck;
use super::interop_finder::InteropFinder;
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use std::ops::Range;
use std::path::Path;
use tachyon_diagnostics::Diagnostic;

pub(crate) struct NativeWebApis;

impl NativeWebApis {
    const LANGUAGES: [(&'static str, &'static str); 5] = [
        ("rs", "Rust"),
        ("cs", "C#"),
        ("fs", "F#"),
        ("swift", "Swift"),
        ("kt", "Kotlin"),
    ];
    const PRELUDE_CLIENTS: [&'static str; 3] = ["rs", "cs", "fs"];
    const HELP: &'static str = "Browser APIs need the page. Use this API in a JavaScript, TypeScript or WebAssembly service, and send the result to this service with an event. See /docs/native-services";

    pub(crate) fn refuse(path: &Path, source: &str) -> Result<(), Failure> {
        Self::refuse_preferences(path, source)?;
        let extension = Self::extension(path);
        let mut names = crate::route_types::WebEventModels::names();
        names.extend(Self::clients(extension));
        let words: Vec<&str> = names.iter().map(String::as_str).collect();
        let uses = InteropFinder::new(extension, source).all_words(&words);
        let language = Self::language(extension);
        let problems = uses
            .into_iter()
            .map(|range| Self::problem(path, source, language, range))
            .collect();
        BrowserInteropCheck::refuse(problems)
    }

    pub(crate) fn refuse_preferences(path: &Path, source: &str) -> Result<(), Failure> {
        BrowserInteropCheck::refuse(super::browser_providers::BrowserProviders::native(
            path, source,
        ))
    }

    fn clients(extension: &str) -> Vec<String> {
        if !Self::PRELUDE_CLIENTS.contains(&extension) {
            return Vec::new();
        }
        crate::web_api_clients::WebApiClients::TYPE_NAMES
            .into_iter()
            .filter(|name| !matches!(extension, "cs" | "kt" | "rs") || *name != "WindowControls")
            .map(String::from)
            .collect()
    }

    fn extension(path: &Path) -> &str {
        path.extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default()
    }

    fn language(extension: &str) -> &'static str {
        Self::LANGUAGES
            .iter()
            .find(|(known, _)| *known == extension)
            .map_or("Native", |(_, language)| language)
    }

    fn problem(path: &Path, source: &str, language: &str, range: Range<usize>) -> Diagnostic {
        let name = &source[range.clone()];
        let message = format!(
            "{language} services that are linked into the app cannot use `{name}`: it belongs to the page."
        );
        let file = path.to_string_lossy().replace('\\', "/");
        diagnostic(
            BrowserInteropCheck::CODE,
            message,
            Some(String::from(Self::HELP)),
            source_span(&file, range.start, range.end),
        )
    }
}
