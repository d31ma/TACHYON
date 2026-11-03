use super::ScaffoldFailure;
use crate::Failure;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) struct TargetFolder;

impl TargetFolder {
    pub(crate) fn absolute(target: &Path) -> Result<PathBuf, Failure> {
        Self::refuse_empty_path(target)?;
        let current = Self::current_folder()?;
        Ok(current.join(target))
    }

    pub(crate) fn check_empty(target: &Path) -> Result<(), Failure> {
        Self::metadata(target)?.map_or(Ok(()), |metadata| Self::check_existing(target, &metadata))
    }

    pub(crate) fn parent(target: &Path) -> Result<&Path, Failure> {
        target.parent().ok_or_else(|| {
            ScaffoldFailure::target(format!(
                "Initialization target '{}' has no parent.",
                target.display()
            ))
        })
    }

    pub(crate) fn canonical(target: &Path) -> Result<PathBuf, Failure> {
        ScaffoldFailure::checked(fs::canonicalize(target), target)
    }

    fn refuse_empty_path(target: &Path) -> Result<(), Failure> {
        if target.as_os_str().is_empty() {
            return Err(ScaffoldFailure::target(String::from(
                "Initialization target cannot be empty.",
            )));
        }
        Ok(())
    }

    fn current_folder() -> Result<PathBuf, Failure> {
        std::env::current_dir().map_err(|error| {
            ScaffoldFailure::target(format!("Cannot resolve initialization target: {error}"))
        })
    }

    fn metadata(target: &Path) -> Result<Option<fs::Metadata>, Failure> {
        match fs::symlink_metadata(target) {
            Ok(metadata) => Ok(Some(metadata)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(ScaffoldFailure::io(target, &error)),
        }
    }

    fn check_existing(target: &Path, metadata: &fs::Metadata) -> Result<(), Failure> {
        Self::refuse_non_folder(target, metadata)?;
        Self::refuse_entries(target)
    }

    fn refuse_non_folder(target: &Path, metadata: &fs::Metadata) -> Result<(), Failure> {
        let folder = [metadata.file_type().is_symlink(), metadata.is_dir()] == [false, true];
        if folder {
            return Ok(());
        }
        Err(ScaffoldFailure::target(format!(
            "Initialization target '{}' must be a regular directory.",
            target.display()
        )))
    }

    fn refuse_entries(target: &Path) -> Result<(), Failure> {
        let mut entries = ScaffoldFailure::checked(fs::read_dir(target), target)?;
        let first = ScaffoldFailure::checked(entries.next().transpose(), target)?;
        first.map_or(Ok(()), |_| Err(ScaffoldFailure::not_empty(target)))
    }
}
