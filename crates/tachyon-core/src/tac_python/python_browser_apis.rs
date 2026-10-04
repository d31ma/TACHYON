use crate::Failure;
use crate::web_api_clients::WebApiClients;
use std::ops::Range;
use std::path::Path;

pub(super) struct PythonBrowserApis;

impl PythonBrowserApis {
    const EXTENSION: &'static str = "py";
    const NAME_JOINER: char = '_';
    const HELP: &'static str = "The Pyodide runtime has no browser API adapter yet. Use this API in a JavaScript, TypeScript or compiled service, and pass the value to the Python service as an argument.";

    pub(super) fn refuse(path: &Path, source: &str) -> Result<(), Failure> {
        Self::first_use(source).map_or(Ok(()), |range| Err(Self::failure(path, source, range)))
    }

    fn failure(path: &Path, source: &str, range: Range<usize>) -> Failure {
        let name = &source[range.clone()];
        Failure::one(crate::failure::diagnostic(
            1605,
            format!("Python Tac services cannot use `{name}`: it belongs to the page."),
            Some(String::from(Self::HELP)),
            crate::failure::source_span(&path.to_string_lossy(), range.start, range.end),
        ))
    }

    fn first_use(source: &str) -> Option<Range<usize>> {
        let code = crate::lexical::code_mask(Self::EXTENSION, source).code;
        WebApiClients::client_type_names()
            .flat_map(|name| {
                source
                    .match_indices(name)
                    .map(|(at, _)| at..at + name.len())
            })
            .filter(|range| Self::is_code_word(source, &code, range))
            .min_by_key(|range| range.start)
    }

    fn is_code_word(source: &str, code: &[bool], range: &Range<usize>) -> bool {
        let before = source[..range.start].chars().next_back();
        let after = source[range.end..].chars().next();
        [
            code.get(range.start).copied().unwrap_or(false),
            !Self::continues_name(before),
            !Self::continues_name(after),
        ] == [true, true, true]
    }

    fn continues_name(character: Option<char>) -> bool {
        character.is_some_and(|character| {
            [character.is_alphanumeric(), character == Self::NAME_JOINER].contains(&true)
        })
    }
}
