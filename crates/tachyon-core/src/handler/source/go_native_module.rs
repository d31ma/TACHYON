use super::staged_files::StagedFiles;
use crate::Failure;
use crate::annotation_layout::NativePackageFolder;
use crate::failure::{diagnostic, source_span};
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub(super) struct GoNativeModule {
    path: Option<String>,
    files: Vec<(PathBuf, String)>,
}

impl GoNativeModule {
    const HANDLER_MODULE: &'static str = "module yonhandler\n\ngo 1.27\n";
    const STAGED_FOLDER: &'static str = "packages";
    const CODE: u16 = 1310;

    pub(super) fn read(source_root: &Path) -> Result<Self, Failure> {
        let root = Path::new(NativePackageFolder::ROOT);
        let files = StagedFiles::beneath(source_root, root)
            .iter()
            .map(|relative| Self::staged_file(source_root, root, relative))
            .collect::<Result<Vec<_>, Failure>>()?;
        let module_file = Path::new(Self::STAGED_FOLDER).join(NativePackageFolder::MODULE_FILE);
        let path = files
            .iter()
            .find(|(name, _)| *name == module_file)
            .and_then(|(_, text)| NativePackageFolder::module_path(text));
        Ok(Self { path, files })
    }

    pub(super) fn module_file(&self) -> String {
        self.path.as_ref().map_or_else(
            || String::from(Self::HANDLER_MODULE),
            |path| {
                format!(
                    "{}\nrequire {path} v0.0.0\n\nreplace {path} => ./{}\n",
                    Self::HANDLER_MODULE,
                    Self::STAGED_FOLDER
                )
            },
        )
    }

    pub(super) fn files(&self) -> &[(PathBuf, String)] {
        &self.files
    }

    fn staged_file(
        source_root: &Path,
        root: &Path,
        relative: &Path,
    ) -> Result<(PathBuf, String), Failure> {
        let text = Self::read_text(source_root, relative)?;
        let beneath = relative.strip_prefix(root).unwrap_or(relative);
        Ok((Path::new(Self::STAGED_FOLDER).join(beneath), text))
    }

    fn read_text(source_root: &Path, relative: &Path) -> Result<String, Failure> {
        let portable = relative.to_string_lossy().replace('\\', "/");
        let bytes = std::fs::read(source_root.join(relative)).map_err(|error| {
            Self::failure(&portable, format!("Cannot read '{portable}': {error}"))
        })?;
        String::from_utf8(bytes).map_err(|_| {
            Self::failure(
                &portable,
                format!("'{portable}' must be UTF-8 to be staged with the native Go module."),
            )
        })
    }

    fn failure(portable: &str, message: String) -> Failure {
        Failure::one(diagnostic(
            Self::CODE,
            message,
            None,
            source_span(portable, 0, 0),
        ))
    }
}
