use super::host::native_tool_failure;
use super::native_reproducibility::NativeReproducibility;
use crate::Failure;
use std::path::{Path, PathBuf};

pub(super) struct SwiftCompileInputs {
    pub(super) folder: PathBuf,
    source: String,
    companion: Option<String>,
    library: Option<String>,
    executable: String,
}

impl SwiftCompileInputs {
    const SOURCE: &'static str = "Swift source path is not valid Unicode.";
    const COMPANION: &'static str = "Companion source path is not valid Unicode.";
    const LIBRARY: &'static str = "Companion library path is not valid Unicode.";
    const EXECUTABLE: &'static str = "Application path is not valid Unicode.";

    pub(super) fn of(
        source: &Path,
        companion: Option<&Path>,
        library: Option<&Path>,
        executable: &Path,
    ) -> Result<Self, Failure> {
        let folder = NativeReproducibility::source_folder(source)?;
        Ok(Self {
            source: Self::text(
                NativeReproducibility::within(&folder, source)?,
                Self::SOURCE,
            )?,
            companion: Self::relative_text(&folder, companion)?,
            library: Self::absolute_text(library)?,
            executable: Self::text(
                NativeReproducibility::absolute(executable)?,
                Self::EXECUTABLE,
            )?,
            folder,
        })
    }

    pub(super) fn arguments(&self) -> Vec<&str> {
        let mut arguments = vec!["-o", self.executable.as_str(), self.source.as_str()];
        arguments.extend(self.companion.as_deref());
        arguments.extend(self.library.as_deref());
        arguments
    }

    fn relative_text(folder: &Path, path: Option<&Path>) -> Result<Option<String>, Failure> {
        path.map(|path| {
            Self::text(
                NativeReproducibility::within(folder, path)?,
                Self::COMPANION,
            )
        })
        .transpose()
    }

    fn absolute_text(path: Option<&Path>) -> Result<Option<String>, Failure> {
        path.map(|path| Self::text(NativeReproducibility::absolute(path)?, Self::LIBRARY))
            .transpose()
    }

    fn text(path: PathBuf, message: &str) -> Result<String, Failure> {
        path.into_os_string()
            .into_string()
            .map_err(|_| native_tool_failure(1605, message))
    }
}
