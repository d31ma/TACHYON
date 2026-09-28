use crate::ScanError;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) struct SourceFiles;

impl SourceFiles {
    const MEMBER_FOLDERS: [&'static str; 2] = ["crates", "tools"];

    pub(crate) fn under(root: &Path) -> Result<Vec<PathBuf>, ScanError> {
        let mut members = Self::members(root)?;
        members.retain(|member| member.is_dir());
        let mut files = Self::build_scripts(&members);
        Self::collect_sources(&members, &mut files)?;
        files.retain(|path| Self::is_rust(path));
        files.retain(|path| !Self::is_test(path));
        files.sort();
        Ok(files)
    }

    pub(crate) fn relative(root: &Path, path: &Path) -> String {
        let relative = path.strip_prefix(root).unwrap_or(path);
        relative.to_string_lossy().replace('\\', "/")
    }

    fn members(root: &Path) -> Result<Vec<PathBuf>, ScanError> {
        let mut members = Vec::new();
        for folder in Self::MEMBER_FOLDERS {
            Self::add_children(&root.join(folder), &mut members)?;
        }
        Ok(members)
    }

    fn add_children(folder: &Path, children: &mut Vec<PathBuf>) -> Result<(), ScanError> {
        for entry in fs::read_dir(folder).map_err(|error| ScanError::new(folder, &error))? {
            children.push(
                entry
                    .map_err(|error| ScanError::new(folder, &error))?
                    .path(),
            );
        }
        Ok(())
    }

    fn build_scripts(members: &[PathBuf]) -> Vec<PathBuf> {
        members
            .iter()
            .map(|member| member.join("build.rs"))
            .filter(|script| script.is_file())
            .collect()
    }

    fn collect_sources(members: &[PathBuf], files: &mut Vec<PathBuf>) -> Result<(), ScanError> {
        let sources = members.iter().map(|member| member.join("src"));
        for source in sources.filter(|source| source.is_dir()) {
            Self::walk(&source, files)?;
        }
        Ok(())
    }

    fn walk(folder: &Path, files: &mut Vec<PathBuf>) -> Result<(), ScanError> {
        let mut children = Vec::new();
        Self::add_children(folder, &mut children)?;
        for child in children {
            Self::add_entry(child, files)?;
        }
        Ok(())
    }

    fn add_entry(path: PathBuf, files: &mut Vec<PathBuf>) -> Result<(), ScanError> {
        if path.is_dir() {
            return Self::walk(&path, files);
        }
        files.push(path);
        Ok(())
    }

    fn is_rust(path: &Path) -> bool {
        path.extension() == Some(OsStr::new("rs"))
    }

    fn is_test(path: &Path) -> bool {
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let in_tests_folder = path.components().any(|part| part.as_os_str() == "tests");
        [stem == "tests", stem.ends_with("_tests"), in_tests_folder].contains(&true)
    }
}
