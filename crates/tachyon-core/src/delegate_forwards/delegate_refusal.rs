use crate::Failure;
use crate::failure::{diagnostic, source_span};
use std::path::Path;

pub(super) struct DelegateRefusal<'a> {
    pub(super) path: &'a Path,
    pub(super) source: &'a str,
}

impl DelegateRefusal<'_> {
    pub(super) fn at(
        &self,
        code: u16,
        (start, length): (usize, usize),
        (message, help): (String, String),
    ) -> Failure {
        let portable = self.path.to_string_lossy().replace('\\', "/");
        let end = (start + length).min(self.source.len());
        Failure::one(diagnostic(
            code,
            message,
            Some(help),
            source_span(&portable, start, end),
        ))
    }
}
