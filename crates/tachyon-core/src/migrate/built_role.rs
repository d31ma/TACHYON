use super::{MigrationFinding, MigrationStatus};
use std::fs;
use std::path::Path;

pub(super) struct BuiltRole {
    pub(super) feature: &'static str,
    pub(super) suffix: &'static str,
    pub(super) detail: &'static str,
    pub(super) action: &'static str,
}

impl BuiltRole {
    const MAX_SOURCE_BYTES: usize = 1024 * 1024;

    pub(super) fn source(root: &Path, relative: &str) -> Option<String> {
        let bytes = fs::read(root.join(relative)).ok()?;
        (bytes.len() <= Self::MAX_SOURCE_BYTES).then_some(())?;
        String::from_utf8(bytes).ok()
    }

    pub(super) fn finding(
        &self,
        relative: &str,
        source: &str,
        builds: fn(&str, usize) -> bool,
    ) -> Option<MigrationFinding> {
        let extension = Path::new(relative).extension()?.to_str()?;
        let mask = crate::lexical::code_mask(extension, source);
        let built = source
            .match_indices(self.suffix)
            .filter(|(at, _)| mask.code.get(*at) == Some(&true))
            .any(|(at, _)| builds(source, at));
        built.then(|| {
            MigrationFinding::new(
                relative,
                self.feature,
                MigrationStatus::Changed,
                self.detail,
                Some(self.action),
            )
        })
    }

    pub(super) fn name_start(before: &str) -> usize {
        before
            .rfind(|character: char| !Self::is_name_character(character))
            .map_or(0, |found| found + 1)
    }

    pub(super) fn is_name_character(character: char) -> bool {
        [character.is_ascii_alphanumeric(), character == '_'].contains(&true)
    }
}
