use super::page_state::PageState;
use super::portable_path::PortablePath;
use crate::Failure;
use crate::ascii_name::AsciiName;
use crate::failure::diagnostic;
use std::collections::BTreeSet;
use std::fs::{self, DirEntry, FileType, Metadata};
use std::path::{Path, PathBuf};
use std::string::FromUtf8Error;

pub(super) struct SharedAssets;

impl SharedAssets {
    const MAX_ASSET_BYTES: u64 = 16 * 1_048_576;
    const MAX_ASSETS: usize = 4_096;
    const MAX_TOTAL_BYTES: u64 = 64 * 1_048_576;
    const UTILITY_ASSETS: &'static str = "shared/utils";

    pub(super) fn collect(project_root: &Path) -> Result<Vec<(PathBuf, Vec<u8>)>, Failure> {
        let root = project_root.join("client/shared");
        let assets = Self::walk_when_present(&root)?;
        let mut assets = super::shared_scripts::SharedScripts::prepare(project_root, assets)?;
        assets.retain(|(path, _)| !path.starts_with(Self::UTILITY_ASSETS));
        assets.sort_by_key(|asset| PortablePath::of(&asset.0));
        Ok(assets)
    }

    pub(super) fn data_names(project_root: &Path) -> BTreeSet<String> {
        fs::read_dir(project_root.join("client/shared/data"))
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| Self::data_name(&entry))
            .collect()
    }

    pub(super) fn edit_text(bytes: Vec<u8>, edit: impl FnOnce(String) -> String) -> Vec<u8> {
        String::from_utf8(bytes)
            .map_or_else(FromUtf8Error::into_bytes, |text| edit(text).into_bytes())
    }

    fn data_name(entry: &DirEntry) -> Option<String> {
        entry.file_type().ok().filter(FileType::is_file)?;
        let name = entry.file_name().into_string().ok()?;
        let stem = name.strip_suffix(".json")?;
        Self::is_data_name(stem).then(|| stem.to_owned())
    }

    fn is_data_name(stem: &str) -> bool {
        let mut bytes = stem.bytes();
        let first = bytes.next().is_some_and(Self::is_name_start);
        let rest = bytes.all(AsciiName::is_name_byte);
        first && rest
    }

    fn is_name_start(byte: u8) -> bool {
        byte.is_ascii_alphabetic() || byte == b'_'
    }

    fn walk_when_present(root: &Path) -> Result<Vec<(PathBuf, Vec<u8>)>, Failure> {
        match fs::symlink_metadata(root) {
            Ok(metadata) => Self::walk_root(root, &metadata),
            Err(_) => Ok(Vec::new()),
        }
    }

    fn walk_root(root: &Path, metadata: &Metadata) -> Result<Vec<(PathBuf, Vec<u8>)>, Failure> {
        Self::refuse(
            !Self::is_real_directory(metadata),
            root,
            "the shared asset root must be a regular directory",
        )?;
        Self::walk(root)
    }

    fn is_real_directory(metadata: &Metadata) -> bool {
        metadata.is_dir() && !metadata.file_type().is_symlink()
    }

    fn walk(root: &Path) -> Result<Vec<(PathBuf, Vec<u8>)>, Failure> {
        let mut directories = vec![root.to_path_buf()];
        let mut assets = Vec::new();
        let mut total = 0_u64;
        while let Some(directory) = directories.pop() {
            Self::read_directory(root, &directory, &mut directories, &mut assets, &mut total)?;
        }
        Ok(assets)
    }

    fn read_directory(
        root: &Path,
        directory: &Path,
        directories: &mut Vec<PathBuf>,
        assets: &mut Vec<(PathBuf, Vec<u8>)>,
        total: &mut u64,
    ) -> Result<(), Failure> {
        let entries = fs::read_dir(directory).map_err(|error| Self::error(directory, &error))?;
        for entry in entries {
            let entry = entry.map_err(|error| Self::error(directory, &error))?;
            Self::visit(root, &entry.path(), directories, assets, total)?;
        }
        Ok(())
    }

    fn visit(
        root: &Path,
        path: &Path,
        directories: &mut Vec<PathBuf>,
        assets: &mut Vec<(PathBuf, Vec<u8>)>,
        total: &mut u64,
    ) -> Result<(), Failure> {
        let metadata = fs::symlink_metadata(path).map_err(|error| Self::error(path, &error))?;
        Self::refuse(
            metadata.file_type().is_symlink(),
            path,
            "symbolic links are not allowed",
        )?;
        Self::add_entry(root, path, &metadata, directories, assets, total)
    }

    fn add_entry(
        root: &Path,
        path: &Path,
        metadata: &Metadata,
        directories: &mut Vec<PathBuf>,
        assets: &mut Vec<(PathBuf, Vec<u8>)>,
        total: &mut u64,
    ) -> Result<(), Failure> {
        if metadata.is_dir() {
            directories.push(path.to_path_buf());
            return Ok(());
        }
        Self::add_asset(root, path, metadata, assets, total)
    }

    fn add_asset(
        root: &Path,
        path: &Path,
        metadata: &Metadata,
        assets: &mut Vec<(PathBuf, Vec<u8>)>,
        total: &mut u64,
    ) -> Result<(), Failure> {
        *total = total.saturating_add(metadata.len());
        Self::check_limits(root, path, metadata, *total, assets.len())?;
        let relative = path
            .strip_prefix(root)
            .map_err(|error| Self::error(path, &error))?;
        let bytes = fs::read(path).map_err(|error| Self::error(path, &error))?;
        assets.push((PathBuf::from("shared").join(relative), bytes));
        Ok(())
    }

    fn check_limits(
        root: &Path,
        path: &Path,
        metadata: &Metadata,
        total: u64,
        count: usize,
    ) -> Result<(), Failure> {
        Self::refuse(!metadata.is_file(), path, "only regular files are allowed")?;
        Self::refuse(
            metadata.len() > Self::MAX_ASSET_BYTES,
            path,
            "the file exceeds the 16 MiB per-asset limit",
        )?;
        Self::refuse(
            total > Self::MAX_TOTAL_BYTES,
            root,
            "the asset tree exceeds the 64 MiB total limit",
        )?;
        Self::refuse(
            count >= Self::MAX_ASSETS,
            root,
            "the asset tree exceeds the 4096-file limit",
        )
    }

    fn refuse(broken: bool, path: &Path, detail: &str) -> Result<(), Failure> {
        if broken {
            return Err(Self::failure(path, detail));
        }
        Ok(())
    }

    pub(super) fn strip_css_imports(path: &Path, bytes: Vec<u8>) -> Vec<u8> {
        let script = Self::is_script(path);
        Self::strip_when_script(script, bytes)
    }

    fn is_script(path: &Path) -> bool {
        path.extension()
            .is_some_and(|extension| extension == "js" || extension == "mjs")
    }

    fn strip_when_script(script: bool, bytes: Vec<u8>) -> Vec<u8> {
        if !script {
            return bytes;
        }
        Self::edit_text(bytes, |source| Self::without_css_imports(&source))
    }

    fn without_css_imports(source: &str) -> String {
        source
            .split_inclusive('\n')
            .filter(|line| !Self::is_css_import(line))
            .collect()
    }

    fn is_css_import(line: &str) -> bool {
        let statement = line.trim().trim_end_matches(';');
        statement
            .strip_prefix("import ")
            .and_then(PageState::unquoted)
            .is_some_and(Self::is_css_path)
    }

    fn is_css_path(specifier: &str) -> bool {
        Path::new(specifier)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("css"))
    }

    fn failure(path: &Path, detail: &str) -> Failure {
        Failure::one(diagnostic(
            1201,
            format!(
                "Cannot publish shared asset '{}': {detail}.",
                path.display()
            ),
            Some(String::from(
                "Keep client/shared inside the project as bounded regular files and directories.",
            )),
            None,
        ))
    }

    fn error(path: &Path, error: &impl std::fmt::Display) -> Failure {
        Self::failure(path, &error.to_string())
    }
}
