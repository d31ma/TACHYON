use super::host::native_io;
use crate::Failure;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::Path;

pub(super) struct NativeBuildFolders;

impl NativeBuildFolders {
    pub(super) const GRADLE: [&'static str; 4] = [".gradle", ".kotlin", "build", "app/build"];
    pub(super) const DOTNET: [&'static str; 2] = ["bin", "obj"];

    pub(super) fn remove(project: &Path, folders: &[&str]) -> Result<(), Failure> {
        folders
            .iter()
            .map(|folder| project.join(folder))
            .try_for_each(|path| Self::removed(fs::remove_dir_all(&path), &path))
    }

    pub(super) fn remove_file(path: &Path) -> Result<(), Failure> {
        Self::removed(fs::remove_file(path), path)
    }

    fn removed(result: io::Result<()>, path: &Path) -> Result<(), Failure> {
        match result {
            Err(error) if error.kind() != ErrorKind::NotFound => native_io(Err(error), path),
            _ => Ok(()),
        }
    }
}
