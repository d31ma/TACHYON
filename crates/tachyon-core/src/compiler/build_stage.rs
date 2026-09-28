use super::content_digest::ContentDigest;
use super::portable_path::PortablePath;
use crate::Failure;
use crate::failure::diagnostic;
use sha2::{Digest, Sha256};
use std::fs::{self, Metadata};
use std::io;
use std::path::{Path, PathBuf};

pub(crate) struct BuildStage;

impl BuildStage {
    const MAX_FILES: usize = 4_096;
    const MAX_FILE_BYTES: u64 = 16 * 1_048_576;
    const MAX_TOTAL_BYTES: u64 = 64 * 1_048_576;
    const MAX_BUILD_CONFIG_BYTES: u64 = 1_048_576;

    pub(super) fn create(output_directory: &Path) -> Result<tempfile::TempDir, Failure> {
        let parent = output_directory.parent().ok_or_else(|| {
            Failure::one(diagnostic(
                1201,
                "The output directory has no writable parent.",
                None,
                None,
            ))
        })?;
        Self::output_io(fs::create_dir_all(parent), output_directory)?;
        Self::output_io(
            tempfile::Builder::new()
                .prefix(".tachyon-build-")
                .tempdir_in(parent),
            output_directory,
        )
    }

    pub(super) fn config_digest(project_root: &Path) -> Result<Option<String>, Failure> {
        let manifest = project_root.join(crate::native::WebAppManifest::SOURCE_PATH);
        let exists = Self::output_io(manifest.try_exists(), &manifest)?;
        exists.then(|| Self::manifest_digest(&manifest)).transpose()
    }

    pub(super) fn write(stage: &Path, files: &[(PathBuf, Vec<u8>)]) -> io::Result<String> {
        let mut hasher = Sha256::new();
        files.iter().try_for_each(|(relative, contents)| {
            Self::write_file(stage, relative, contents, &mut hasher)
        })?;
        Ok(ContentDigest::hex(hasher.finalize()))
    }

    pub(super) fn digest(stage: &Path) -> io::Result<String> {
        let files = Self::sorted_files(stage)?;
        Self::hash_files(stage, &files)
    }

    pub(crate) fn publish(stage: tempfile::TempDir, destination: &Path) -> io::Result<()> {
        let persisted = stage.keep();
        Self::move_into_place(&persisted, destination)
    }

    pub(super) fn output_error(path: &Path, error: &io::Error) -> Failure {
        Failure::one(diagnostic(
            1201,
            format!("Cannot publish build output '{}': {error}", path.display()),
            Some(String::from(
                "Check directory permissions and that no process has locked the output.",
            )),
            None,
        ))
    }

    pub(super) fn output_io<T>(result: io::Result<T>, path: &Path) -> Result<T, Failure> {
        result.map_err(|error| Self::output_error(path, &error))
    }

    fn manifest_digest(manifest: &Path) -> Result<String, Failure> {
        let bytes = Self::read_build_config_input(manifest)?;
        let mut digest_input = Vec::new();
        digest_input.extend_from_slice(b"manifest.json");
        digest_input.push(0);
        digest_input.extend_from_slice(&bytes);
        digest_input.push(0);
        Ok(ContentDigest::sha256_bytes(&digest_input))
    }

    fn read_build_config_input(path: &Path) -> Result<Vec<u8>, Failure> {
        let metadata = Self::output_io(fs::symlink_metadata(path), path)?;
        if metadata.file_type().is_symlink()
            || !metadata.is_file()
            || metadata.len() > Self::MAX_BUILD_CONFIG_BYTES
        {
            return Err(Failure::one(diagnostic(
                1201,
                "client/manifest.json must be a regular non-symlinked file no larger than 1 MiB.",
                None,
                None,
            )));
        }
        Self::output_io(fs::read(path), path)
    }

    fn write_file(
        stage: &Path,
        relative: &Path,
        contents: &[u8],
        hasher: &mut Sha256,
    ) -> io::Result<()> {
        let output = stage.join(relative);
        Self::create_parent(&output)?;
        fs::write(&output, contents)?;
        let portable = relative.to_string_lossy().replace('\\', "/");
        hasher.update(portable.as_bytes());
        hasher.update([0]);
        hasher.update(contents);
        hasher.update([0]);
        Ok(())
    }

    fn create_parent(output: &Path) -> io::Result<()> {
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(())
    }

    fn sorted_files(stage: &Path) -> io::Result<Vec<PathBuf>> {
        let mut files = Self::collect_files(stage)?;
        if files.len() > Self::MAX_FILES {
            return Err(Self::invalid_stage("Build output exceeds 4,096 files"));
        }
        files.sort();
        Ok(files)
    }

    fn hash_files(stage: &Path, files: &[PathBuf]) -> io::Result<String> {
        let mut total = 0_u64;
        let mut hasher = Sha256::new();
        for path in files {
            Self::hash_file(stage, path, &mut total, &mut hasher)?;
        }
        Ok(ContentDigest::hex(hasher.finalize()))
    }

    fn hash_file(
        stage: &Path,
        path: &Path,
        total: &mut u64,
        hasher: &mut Sha256,
    ) -> io::Result<()> {
        let bytes = fs::read(path)?;
        *total = total.saturating_add(bytes.len() as u64);
        Self::check_total(*total)?;
        let relative = path.strip_prefix(stage).map_err(io::Error::other)?;
        hasher.update(PortablePath::of(relative).as_bytes());
        hasher.update([0]);
        hasher.update(&bytes);
        hasher.update([0]);
        Ok(())
    }

    fn check_total(total: u64) -> io::Result<()> {
        if total > Self::MAX_TOTAL_BYTES {
            return Err(Self::invalid_stage("Build output exceeds 64 MiB"));
        }
        Ok(())
    }

    fn collect_files(stage: &Path) -> io::Result<Vec<PathBuf>> {
        let mut pending = vec![stage.to_path_buf()];
        let mut files = Vec::new();
        while let Some(current) = pending.pop() {
            Self::visit(current, &mut pending, &mut files)?;
        }
        Ok(files)
    }

    fn visit(
        current: PathBuf,
        pending: &mut Vec<PathBuf>,
        files: &mut Vec<PathBuf>,
    ) -> io::Result<()> {
        let metadata = fs::symlink_metadata(&current)?;
        Self::refuse_symlink(&metadata)?;
        pending.extend(Self::children(&current, &metadata)?);
        files.extend(Self::regular_file(current, &metadata)?);
        Ok(())
    }

    fn refuse_symlink(metadata: &Metadata) -> io::Result<()> {
        if metadata.file_type().is_symlink() {
            return Err(Self::invalid_stage(
                "Build output may not contain symbolic links",
            ));
        }
        Ok(())
    }

    fn children(current: &Path, metadata: &Metadata) -> io::Result<Vec<PathBuf>> {
        if !metadata.is_dir() {
            return Ok(Vec::new());
        }
        fs::read_dir(current)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect()
    }

    fn regular_file(current: PathBuf, metadata: &Metadata) -> io::Result<Option<PathBuf>> {
        let file = metadata.is_file().then_some(current);
        file.map(|file| Self::within_file_limit(file, metadata))
            .transpose()
    }

    fn within_file_limit(file: PathBuf, metadata: &Metadata) -> io::Result<PathBuf> {
        if metadata.len() > Self::MAX_FILE_BYTES {
            return Err(Self::invalid_stage("Build output file exceeds 16 MiB"));
        }
        Ok(file)
    }

    fn invalid_stage(message: &'static str) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidData, message)
    }

    fn move_into_place(persisted: &Path, destination: &Path) -> io::Result<()> {
        if !destination.exists() {
            return fs::rename(persisted, destination);
        }
        Self::replace_existing(persisted, destination)
    }

    fn replace_existing(persisted: &Path, destination: &Path) -> io::Result<()> {
        let backup = persisted.with_extension("backup");
        fs::rename(destination, &backup)?;
        fs::rename(persisted, destination)
            .inspect_err(|_| Self::roll_back(persisted, destination, &backup))?;
        fs::remove_dir_all(&backup)
            .inspect_err(|_| Self::restore_backup(persisted, destination, &backup))
    }

    fn roll_back(persisted: &Path, destination: &Path, backup: &Path) {
        let _rollback = fs::rename(backup, destination);
        let _cleanup = fs::remove_dir_all(persisted);
    }

    fn restore_backup(persisted: &Path, destination: &Path, backup: &Path) {
        let move_new = fs::rename(destination, persisted);
        let restore_old = fs::rename(backup, destination);
        if move_new.is_ok() && restore_old.is_ok() {
            let _cleanup = fs::remove_dir_all(persisted);
        }
    }
}
