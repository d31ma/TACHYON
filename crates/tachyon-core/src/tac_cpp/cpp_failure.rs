use crate::Failure;
use crate::failure::{diagnostic, source_span};
use std::path::Path;

pub(crate) struct CppFailure;

impl CppFailure {
    pub(crate) const UNREADABLE: u16 = 1371;
    pub(crate) const PENDING: u16 = 1372;
    const HELP: &'static str = "C++ Tac files are header-only .hpp files with one class each (ADR 0099). Tachyon markers are PascalCase standard attributes such as `class [[Service]] OrderService` and `[[Needs]] OrderStore& orders;`.";

    pub(crate) fn unreadable(path: &Path, at: usize, message: &str) -> Failure {
        Self::coded(Self::UNREADABLE, path, at, message)
    }

    pub(crate) fn pending(path: &Path, at: usize, message: &str) -> Failure {
        Self::coded(Self::PENDING, path, at, message)
    }

    pub(crate) fn coded(code: u16, path: &Path, at: usize, message: &str) -> Failure {
        let portable = path.to_string_lossy().replace('\\', "/");
        Failure::one(diagnostic(
            code,
            format!("{message} ({portable})"),
            Some(String::from(Self::HELP)),
            source_span(&portable, at, at + 1),
        ))
    }
}
