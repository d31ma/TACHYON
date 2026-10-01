use super::runtime_assets::RuntimeAssets;
use crate::Failure;
use std::io::Read as _;
use std::path::{Path, PathBuf};

pub(super) struct RuntimeRoots;

impl RuntimeRoots {
    const SOURCE_BYTES: u64 = 1_048_576;

    pub(super) fn present(root: &Path) -> Result<bool, Failure> {
        let sources =
            crate::annotation_modules::Discovery::role_paths(root, Path::new("client/services"))?;
        Self::validated(root, &sources)
    }

    fn validated(root: &Path, paths: &[PathBuf]) -> Result<bool, Failure> {
        let mut present = false;
        for path in paths.iter().filter(|path| Self::is_python(path)) {
            let relative = path.strip_prefix(root).unwrap_or(path);
            let source = Self::source(&root.join(path))?;
            crate::annotation_layout::validate(relative, &source)?;
            present = true;
        }
        Ok(present)
    }

    fn is_python(path: &Path) -> bool {
        path.extension().is_some_and(|extension| extension == "py")
    }

    fn source(path: &Path) -> Result<String, Failure> {
        let file = std::fs::File::open(path).map_err(|error| {
            RuntimeAssets::failure(
                &path.to_string_lossy(),
                &format!("cannot inspect Python source: {error}"),
            )
        })?;
        let mut bytes = Vec::new();
        file.take(Self::SOURCE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| {
                RuntimeAssets::failure(
                    &path.to_string_lossy(),
                    &format!("cannot inspect Python source: {error}"),
                )
            })?;
        Self::bounded(path, &bytes)?;
        String::from_utf8(bytes).map_err(|error| {
            RuntimeAssets::failure(
                &path.to_string_lossy(),
                &format!("Python source must be UTF-8: {error}"),
            )
        })
    }

    fn bounded(path: &Path, bytes: &[u8]) -> Result<(), Failure> {
        if bytes.len() as u64 > Self::SOURCE_BYTES {
            return Err(RuntimeAssets::failure(
                &path.to_string_lossy(),
                "Python service source exceeds 1 MiB",
            ));
        }
        Ok(())
    }
}
