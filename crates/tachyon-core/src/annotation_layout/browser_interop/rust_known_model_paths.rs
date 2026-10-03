use crate::Failure;
use crate::failure::{diagnostic, source_span};
use crate::model_sources::ModelSources;
use crate::route_types::ModelTable;
use std::ops::Range;
use std::path::Path;

pub(super) struct RustKnownModelPaths {
    models: ModelTable,
}

impl RustKnownModelPaths {
    const LANGUAGE: &'static str = "rs";
    const REALM: &'static str = "client";
    const PREFIX: &'static str = "crate::models::";
    const SEPARATORS: [char; 2] = ['_', ':'];

    pub(super) fn retain(
        root: &Path,
        source: &str,
        uses: Vec<Range<usize>>,
    ) -> Result<Vec<Range<usize>>, Failure> {
        Self::captured(root, source, uses)
    }

    fn captured(
        root: &Path,
        source: &str,
        uses: Vec<Range<usize>>,
    ) -> Result<Vec<Range<usize>>, Failure> {
        let _validated = ModelSources::read(root, Self::REALM, Self::LANGUAGE)?;
        let models = ModelTable::read_realm(root, Self::REALM, Self::LANGUAGE)
            .map_err(|message| Self::failure(root, message))?;
        Ok(Self { models }.unresolved(source, uses))
    }

    fn unresolved(&self, source: &str, uses: Vec<Range<usize>>) -> Vec<Range<usize>> {
        uses.into_iter()
            .filter(|range| !self.known(source, range))
            .collect()
    }

    fn known(&self, source: &str, range: &Range<usize>) -> bool {
        let captured = self.models.contains(&source[range.clone()]);
        let qualified = Self::path_start(source, range.start)
            .map(|start| Self::boundary(source, start))
            .unwrap_or(false);
        captured && qualified
    }

    fn path_start(source: &str, end: usize) -> Option<usize> {
        let before = source.get(..end)?.strip_suffix(Self::PREFIX)?;
        Some(before.strip_suffix("::").unwrap_or(before).len())
    }

    fn boundary(source: &str, start: usize) -> bool {
        source[..start]
            .chars()
            .next_back()
            .map(Self::separator)
            .unwrap_or(true)
    }

    fn separator(character: char) -> bool {
        !character.is_alphanumeric() && !Self::SEPARATORS.contains(&character)
    }

    fn failure(root: &Path, message: String) -> Failure {
        let file = root.join(Self::REALM).join("models");
        Failure::one(diagnostic(
            1310,
            message,
            None,
            source_span(&file.to_string_lossy(), 0, 1),
        ))
    }
}
