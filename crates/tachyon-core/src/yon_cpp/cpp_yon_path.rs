use super::CppYonController;
use crate::Failure;
use crate::tac_cpp::{CppAttribute, CppFailure};
use std::path::Path;

pub(crate) struct CppYonPath;

impl CppYonPath {
    const QUOTE: char = '"';
    const FORBIDDEN: [char; 2] = ['"', '\\'];

    pub(crate) fn of(path: &Path, marker: &CppAttribute) -> Result<String, Failure> {
        let text = marker.arguments.trim();
        let empty = text.is_empty().then(String::new);
        empty
            .or_else(|| Self::literal(text))
            .ok_or_else(|| Self::failure(path, marker))
    }

    pub(crate) fn literal(text: &str) -> Option<String> {
        text.strip_prefix(Self::QUOTE)
            .and_then(|rest| rest.strip_suffix(Self::QUOTE))
            .filter(|inner| !inner.contains(Self::FORBIDDEN))
            .map(str::to_owned)
    }

    fn failure(path: &Path, marker: &CppAttribute) -> Failure {
        let name = &marker.name;
        let message = format!(
            "[[{name}]] takes one path in quotes, for example [[{name}(\"/orders\")]]. The path says which URL the code answers"
        );
        CppFailure::coded(CppYonController::CODE, path, marker.at, &message)
    }
}
