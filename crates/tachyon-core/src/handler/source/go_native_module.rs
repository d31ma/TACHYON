use super::staged_files::StagedFiles;
use super::{CompilerOutput, Failure, invalid_source};
use crate::annotation_layout::{NativeModuleVendor, NativePackageFolder};
use crate::failure::{diagnostic, source_span};
use sha2::{Digest as _, Sha256};
use std::path::{Path, PathBuf};

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;

#[derive(Debug, Default)]
pub(super) struct GoNativeModule {
    path: Option<String>,
    files: Vec<(PathBuf, Vec<u8>)>,
}

impl GoNativeModule {
    const HANDLER_MODULE: &'static str = "module yonhandler\n\ngo 1.27\n";
    const INVOCATION_IMPORT: &'static str = "\"yonhandler/yoninvocation\"";
    const INVOCATION_PACKAGE: &'static str = "yoninvocation";
    const CODE: u16 = 1310;
    const VENDOR_PROBLEMS: [&'static str; 2] = [
        "inconsistent vendoring",
        "import lookup disabled by -mod=vendor",
    ];

    pub(super) fn read(source_root: &Path) -> Result<Self, Failure> {
        let root = Path::new(NativePackageFolder::ROOT);
        let files = StagedFiles::beneath(source_root, root)
            .iter()
            .map(|relative| Self::staged_file(source_root, root, relative))
            .collect::<Result<Vec<_>, Failure>>()?;
        let path = files
            .iter()
            .find(|(name, _)| name.as_path() == Path::new(NativePackageFolder::MODULE_FILE))
            .and_then(|(_, bytes)| std::str::from_utf8(bytes).ok())
            .and_then(NativePackageFolder::module_path);
        Ok(Self { path, files })
    }

    pub(super) fn program_folder(&self) -> &str {
        self.path
            .as_ref()
            .map_or("", |_| NativePackageFolder::PROGRAM_FOLDER)
    }

    pub(super) fn runtime(&self, runtime: &str) -> String {
        self.path.as_ref().map_or_else(
            || runtime.to_owned(),
            |path| runtime.replacen(Self::INVOCATION_IMPORT, &Self::invocation_import(path), 1),
        )
    }

    pub(super) fn fingerprint(&self) -> String {
        let hash = self
            .staged()
            .iter()
            .fold(Sha256::new(), |hash, (relative, bytes)| {
                hash.chain_update(relative.to_string_lossy().as_bytes())
                    .chain_update([0])
                    .chain_update(bytes)
                    .chain_update([0])
            });
        crate::compiler::hex_digest(hash.finalize())
    }

    pub(super) fn write(&self, directory: &Path, portable: &str) -> Result<(), Failure> {
        self.staged()
            .iter()
            .try_for_each(|(relative, bytes)| Self::write_file(&directory.join(relative), bytes))
            .map_err(|error| {
                invalid_source(
                    portable,
                    &format!("Cannot stage the native Go module: {error}"),
                )
            })
    }

    pub(super) fn check_build(
        output: &CompilerOutput,
        directory: &Path,
        portable: &str,
    ) -> Result<(), Failure> {
        let canonical = directory.canonicalize().unwrap_or_default();
        let detail = [canonical.as_path(), directory]
            .iter()
            .filter(|place| !place.as_os_str().is_empty())
            .fold(
                String::from_utf8_lossy(&output.stderr).into_owned(),
                |detail, place| {
                    detail.replace(&*place.to_string_lossy(), NativePackageFolder::ROOT)
                },
            );
        Self::is_vendor_problem(output, &detail)
            .then(|| {
                NativeModuleVendor::failure(
                    portable,
                    format!("The Go build of '{portable}' needs modules that server/packages/vendor/ does not provide: {}", detail.trim()),
                )
            })
            .map_or(Ok(()), Err)
    }

    fn invocation_import(path: &str) -> String {
        format!(
            "\"{path}/{}/{}\"",
            NativePackageFolder::PROGRAM_FOLDER,
            Self::INVOCATION_PACKAGE
        )
    }

    fn is_vendor_problem(output: &CompilerOutput, detail: &str) -> bool {
        !output.status.success()
            && Self::VENDOR_PROBLEMS
                .iter()
                .any(|problem| detail.contains(problem))
    }

    fn staged(&self) -> Vec<(&Path, &[u8])> {
        match &self.path {
            None => vec![(
                Path::new(NativePackageFolder::MODULE_FILE),
                Self::HANDLER_MODULE.as_bytes(),
            )],
            Some(_) => self
                .files
                .iter()
                .map(|(relative, bytes)| (relative.as_path(), bytes.as_slice()))
                .collect(),
        }
    }

    fn write_file(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
        path.parent().map_or(Ok(()), std::fs::create_dir_all)?;
        std::fs::write(path, bytes)
    }

    fn staged_file(
        source_root: &Path,
        root: &Path,
        relative: &Path,
    ) -> Result<(PathBuf, Vec<u8>), Failure> {
        let portable = relative.to_string_lossy().replace('\\', "/");
        let bytes = std::fs::read(source_root.join(relative)).map_err(|error| {
            Failure::one(diagnostic(
                Self::CODE,
                format!("Cannot read '{portable}': {error}"),
                None,
                source_span(&portable, 0, 0),
            ))
        })?;
        let beneath = relative.strip_prefix(root).unwrap_or(relative);
        Ok((beneath.to_path_buf(), bytes))
    }
}
