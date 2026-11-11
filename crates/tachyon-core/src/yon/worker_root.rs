use crate::Failure;
use crate::failure::{diagnostic, source_span};
use crate::python_parameters::PythonAnnotated;
use std::path::Path;

pub(crate) struct WorkerRoot;

impl WorkerRoot {
    pub(crate) const NAME: &'static str = "YonWork";
    const AUTHORED: &'static str = "Handler";
    const FOLDER: &'static str = "server/workers";
    const COMMENT_STARTS: [&'static str; 2] = ["// ", "-- "];
    const MARKER_STARTS: [&'static str; 4] = ["#[", "[<", "[", "@"];
    const QUALIFIER: &'static str = "yon::";
    const NEEDS: [&'static str; 2] = ["needs", "needsall"];
    const FSHARP_CONSTRUCTOR: &'static str = "type Handler(";
    const MEMBER_ACCESS: [u8; 3] = *b".$>";
    const TAKEN: &'static str = "A worker that declares Needs cannot use the name YonWork";
    const TAKEN_HELP: &'static str = "Tachyon gives the name YonWork to the Handler class of a worker that declares Needs, so it can build the worker like a controller. Rename your YonWork.";

    pub(crate) fn is_root(path: &Path, name: &str) -> bool {
        [Self::is_worker(path), name == Self::NAME] == [true, true]
    }

    pub(crate) fn injects(path: &Path, source: &str) -> bool {
        [Self::is_worker(path), Self::declares_needs(path, source)] == [true, true]
    }

    pub(crate) fn renamed(path: &Path, source: &str) -> Result<Option<String>, Failure> {
        Self::injects(path, source)
            .then(|| Self::rename(path, source))
            .transpose()
    }

    fn is_worker(path: &Path) -> bool {
        path.starts_with(Self::FOLDER)
    }

    fn declares_needs(path: &Path, source: &str) -> bool {
        let found = [
            source.lines().any(Self::is_needs_marker),
            Self::has_python_needs(path, source),
            Self::has_constructor_needs(path, source),
        ];
        found.contains(&true)
    }

    fn is_needs_marker(line: &str) -> bool {
        Self::NEEDS.contains(&Self::marker_name(line).as_str())
    }

    fn marker_name(line: &str) -> String {
        let trimmed = line.trim_start();
        let uncommented = Self::without_any(trimmed, &Self::COMMENT_STARTS).unwrap_or(trimmed);
        let marked = Self::without_any(uncommented, &Self::MARKER_STARTS).unwrap_or_default();
        let name = marked.strip_prefix(Self::QUALIFIER).unwrap_or(marked);
        name.bytes()
            .take_while(|byte| Self::joins_word(*byte))
            .filter(|byte| *byte != b'_')
            .map(|byte| char::from(byte.to_ascii_lowercase()))
            .collect()
    }

    fn without_any<'a>(text: &'a str, starts: &[&str]) -> Option<&'a str> {
        starts.iter().find_map(|start| text.strip_prefix(start))
    }

    fn has_python_needs(path: &Path, source: &str) -> bool {
        let python = path.extension().is_some_and(|extension| extension == "py");
        python && Self::annotates_needs(path, source)
    }

    fn annotates_needs(path: &Path, source: &str) -> bool {
        let code = crate::annotation_layout::masked_source(path, source).unwrap_or_default();
        let found = PythonAnnotated::markers(&code, &code, |written| {
            Self::NEEDS.contains(&written.replace('_', "").to_ascii_lowercase().as_str())
        });
        !found.markers.is_empty()
    }

    fn has_constructor_needs(path: &Path, source: &str) -> bool {
        let fsharp = path.extension().is_some_and(|extension| extension == "fs");
        fsharp && source.lines().any(Self::takes_constructor_values)
    }

    fn takes_constructor_values(line: &str) -> bool {
        line.trim_start()
            .strip_prefix(Self::FSHARP_CONSTRUCTOR)
            .is_some_and(|rest| !rest.trim_start().starts_with(')'))
    }

    fn rename(path: &Path, source: &str) -> Result<String, Failure> {
        let code = crate::annotation_layout::masked_source(path, source)?;
        Self::refuse_taken(path, &code)?;
        Ok(Self::replaced(source, &code))
    }

    fn refuse_taken(path: &Path, code: &str) -> Result<(), Failure> {
        Self::word_starts(code, Self::NAME)
            .first()
            .map_or(Ok(()), |at| Err(Self::taken(path, *at)))
    }

    fn taken(path: &Path, at: usize) -> Failure {
        let portable = path.to_string_lossy().replace('\\', "/");
        Failure::one(diagnostic(
            2015,
            Self::TAKEN,
            Some(String::from(Self::TAKEN_HELP)),
            source_span(&portable, at, at + Self::NAME.len()),
        ))
    }

    fn replaced(source: &str, code: &str) -> String {
        let mut output = source.to_owned();
        for at in Self::word_starts(code, Self::AUTHORED) {
            output.replace_range(at..at + Self::AUTHORED.len(), Self::NAME);
        }
        output
    }

    fn word_starts(code: &str, word: &str) -> Vec<usize> {
        code.match_indices(word)
            .map(|(at, _)| at)
            .filter(|at| Self::stands_alone(code.as_bytes(), *at, word.len()))
            .collect()
    }

    fn stands_alone(code: &[u8], at: usize, length: usize) -> bool {
        let before = at.checked_sub(1).and_then(|index| code.get(index));
        let after = code.get(at + length);
        let joined = [
            before.is_some_and(|byte| Self::joins_word(*byte)),
            before.is_some_and(|byte| Self::MEMBER_ACCESS.contains(byte)),
            after.is_some_and(|byte| Self::joins_word(*byte)),
        ];
        !joined.contains(&true)
    }

    fn joins_word(byte: u8) -> bool {
        byte.is_ascii_alphanumeric() || byte == b'_'
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
#[path = "worker_root_tests.rs"]
mod tests;
