use crate::handler::{HandlerLanguage, HandlerSource};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HandlerNode {
    pub(super) source_path: String,
    absolute_source_path: PathBuf,
    language: HandlerLanguage,
    source: HandlerSource,
}

impl HandlerNode {
    pub(super) fn of(source: &HandlerSource) -> Self {
        Self {
            source_path: String::from(source.relative_path()),
            absolute_source_path: source.absolute_path().to_path_buf(),
            language: source.language(),
            source: source.clone(),
        }
    }

    #[must_use]
    pub fn source_path(&self) -> &str {
        &self.source_path
    }

    #[must_use]
    pub fn absolute_source_path(&self) -> &Path {
        &self.absolute_source_path
    }

    #[must_use]
    pub const fn language(&self) -> HandlerLanguage {
        self.language
    }

    pub(crate) fn source(&self) -> &HandlerSource {
        &self.source
    }

    pub(super) fn manifest_handler(&self) -> tachyon_contracts::RouteHandler {
        tachyon_contracts::RouteHandler {
            source: self.source_path.clone(),
            language: String::from(self.language.name()),
            runtime: String::from(self.language.adapter()),
        }
    }
}
