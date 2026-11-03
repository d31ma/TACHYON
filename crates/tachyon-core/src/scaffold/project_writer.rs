use super::{ClientFiles, ScaffoldFailure, TargetFolder};
use crate::Failure;
use crate::compiler::publish;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub(crate) struct ProjectWriter;

impl ProjectWriter {
    const STAGE_PREFIX: &'static str = ".tachyon-init-";

    pub(crate) fn publish(target: &Path, files: &BTreeMap<String, String>) -> Result<(), Failure> {
        let parent = TargetFolder::parent(target)?;
        ScaffoldFailure::checked(fs::create_dir_all(parent), target)?;
        let stage = ScaffoldFailure::checked(
            tempfile::Builder::new()
                .prefix(Self::STAGE_PREFIX)
                .tempdir_in(parent),
            target,
        )?;
        Self::write_all(stage.path(), files, target)?;
        ScaffoldFailure::checked(publish(stage, target), target)
    }

    pub(crate) fn write_all(
        root: &Path,
        files: &BTreeMap<String, String>,
        target: &Path,
    ) -> Result<(), Failure> {
        files.iter().try_for_each(|(relative, contents)| {
            Self::write_one(&root.join(relative), contents.as_bytes(), target)
        })?;
        ClientFiles::images_for(files)
            .iter()
            .try_for_each(|(relative, bytes)| Self::write_one(&root.join(relative), bytes, target))
    }

    fn write_one(output: &Path, contents: &[u8], target: &Path) -> Result<(), Failure> {
        let folder = output.parent().unwrap_or(output);
        ScaffoldFailure::checked(fs::create_dir_all(folder), target)?;
        ScaffoldFailure::checked(fs::write(output, contents), target)
    }
}
