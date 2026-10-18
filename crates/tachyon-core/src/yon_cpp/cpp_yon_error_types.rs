use super::CppYonErrorType;
use crate::Failure;
use crate::tac_cpp::{CppFailure, CppFolder, CppProgram, CppSource};
use std::path::Path;

pub(crate) struct CppYonErrorTypes {
    ordered: Vec<(CppYonErrorType, usize)>,
}

impl CppYonErrorTypes {
    const FOLDER: &'static str = "server/errors";
    const STANDARD: [(&'static str, usize); 10] = [
        ("std::exception", 0),
        ("std::runtime_error", 1),
        ("std::logic_error", 1),
        ("std::invalid_argument", 2),
        ("std::domain_error", 2),
        ("std::length_error", 2),
        ("std::out_of_range", 2),
        ("std::range_error", 2),
        ("std::overflow_error", 2),
        ("std::underflow_error", 2),
    ];
    pub(crate) const DERIVED: &'static str = "A C++ error type derives from std::exception, from a standard error such as std::runtime_error, or from another error type in server/errors, so that a [[HandlesError]] method can catch it";

    pub(crate) fn either(
        path: &Path,
        source: &str,
        other: fn(&Path, &str) -> Result<(), Failure>,
    ) -> Result<(), Failure> {
        [other, Self::check_source][usize::from(CppSource::is_cpp(path))](path, source)
    }

    fn check_source(path: &Path, source: &str) -> Result<(), Failure> {
        CppYonErrorType::read(path, source).map(drop)
    }

    pub(crate) fn read(root: &Path) -> Result<Self, Failure> {
        let waiting = CppFolder::files(root, Self::FOLDER)?
            .iter()
            .map(|path| CppFolder::read(root, path))
            .map(|read| read.and_then(|(path, source)| CppYonErrorType::read(&path, &source)))
            .collect::<Result<Vec<_>, _>>()?;
        let mut types = Self {
            ordered: Vec::new(),
        };
        types.order(waiting)?;
        Ok(types)
    }

    pub(crate) fn depth(&self, name: &str) -> Option<usize> {
        let name = name.trim_start_matches("::");
        let standard = Self::STANDARD
            .iter()
            .find(|(written, _)| *written == name)
            .map(|(_, depth)| *depth);
        let own = self
            .ordered
            .iter()
            .find(|(found, _)| found.name == name)
            .map(|(_, depth)| *depth);
        standard.or(own)
    }

    pub(crate) fn located(&self) -> String {
        self.ordered
            .iter()
            .map(|(found, _)| CppProgram::located(&found.path, &found.lowered))
            .collect()
    }

    fn order(&mut self, mut waiting: Vec<CppYonErrorType>) -> Result<(), Failure> {
        while !waiting.is_empty() {
            let ready = waiting
                .iter()
                .position(|found| self.base_depth(found).is_some());
            let found = waiting.remove(ready.unwrap_or(0));
            let depth = self
                .base_depth(&found)
                .ok_or_else(|| CppFailure::unreadable(&found.path, found.at, Self::DERIVED))?;
            self.ordered.push((found, depth + 1));
        }
        Ok(())
    }

    fn base_depth(&self, found: &CppYonErrorType) -> Option<usize> {
        found
            .bases
            .iter()
            .map(|base| self.depth(base))
            .collect::<Option<Vec<usize>>>()
            .and_then(|depths| depths.into_iter().max())
    }
}
