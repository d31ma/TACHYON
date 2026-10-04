use crate::Failure;
use crate::web_api_clients::WebApiClients;
use std::ops::Range;
use std::path::Path;

pub(super) struct PythonBrowserApis;

impl PythonBrowserApis {
    const EXTENSION: &'static str = "py";
    const HELP: &'static str = "The Pyodide runtime has no browser API adapter yet. Use this API in a JavaScript, TypeScript or compiled service, and pass the value to the Python service as an argument.";

    pub(super) fn refuse(path: &Path, source: &str) -> Result<(), Failure> {
        let Some(range) = Self::first_use(source) else {
            return Ok(());
        };
        let name = &source[range.clone()];
        Err(Failure::one(crate::failure::diagnostic(
            1605,
            format!("Python Tac services cannot use `{name}`: it belongs to the page."),
            Some(String::from(Self::HELP)),
            crate::failure::source_span(&path.to_string_lossy(), range.start, range.end),
        )))
    }

    fn first_use(source: &str) -> Option<Range<usize>> {
        let code = crate::lexical::code_mask(Self::EXTENSION, source).code;
        let mut start = None;
        for (at, character) in source.char_indices().chain([(source.len(), ' ')]) {
            let in_word = code.get(at).copied().unwrap_or(false)
                && (character.is_alphanumeric() || character == '_');
            match (start, in_word) {
                (None, true) => start = Some(at),
                (Some(begin), false) => {
                    if WebApiClients::is_client_type(&source[begin..at]) {
                        return Some(begin..at);
                    }
                    start = None;
                }
                _ => {}
            }
        }
        None
    }
}
