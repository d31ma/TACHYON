use super::BrowserInteropCheck;
use super::interop_finder::InteropFinder;
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use std::ops::Range;
use std::path::Path;
use tachyon_diagnostics::Diagnostic;

pub(crate) struct NativeWebApis;

impl NativeWebApis {
    const LANGUAGES: [(&'static str, &'static str); 4] = [
        ("rs", "Rust"),
        ("cs", "C#"),
        ("swift", "Swift"),
        ("kt", "Kotlin"),
    ];
    const PRELUDE_CLIENTS: [&'static str; 2] = ["rs", "cs"];
    const HELP: &'static str = "Browser APIs need the page. Use this API in a JavaScript, TypeScript or WebAssembly service, and send the result to this service with an event. See /docs/native-services";

    pub(crate) fn refuse(path: &Path, source: &str) -> Result<(), Failure> {
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

    fn clients(extension: &str) -> Vec<String> {
        if !Self::PRELUDE_CLIENTS.contains(&extension) {
            return Vec::new();
        }
        crate::web_api_clients::WebApiClients::TYPE_NAMES
            .into_iter()
