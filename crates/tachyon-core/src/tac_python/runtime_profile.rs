use super::runtime_assets::RuntimeAssets;
use crate::Failure;
use serde_json::Value;
use std::io::Read as _;
use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RuntimeProfile;

impl RuntimeProfile {
    pub(crate) const SCRIPT_BYTES: u64 = 512 * 1024;
    const DEFAULT_ASSET_BYTES: usize = 4 * 1024 * 1024;
    const WASM_ASSET_BYTES: usize = 10 * 1024 * 1024;

    pub(crate) fn read(manifest: &Value) -> Result<Option<Self>, Failure> {
        Self::selected(manifest.pointer("/tac/python"))
    }

    pub(crate) fn for_project(root: &Path) -> Result<Option<Self>, Failure> {
        Self::read(&Self::manifest(root)?)
    }

    pub(crate) fn for_files(
        root: &Path,
        files: &[(std::path::PathBuf, Vec<u8>)],
    ) -> Result<Option<Self>, Failure> {
        let selected = Self::for_project(root)?;
        Ok(selected.filter(|_| crate::compiler::BudgetCheck::has_python_roots(files)))
    }

    pub(crate) fn asset_limit(self, path: &str, bytes: &[u8]) -> Result<usize, Failure> {
        Self::limit(path, bytes)
    }

    fn manifest(root: &Path) -> Result<Value, Failure> {
        let path = root.join("client/manifest.json");
        let file = Self::manifest_file(&path)?;
        file.map(Self::manifest_value)
            .transpose()
            .map(|value| value.unwrap_or(Value::Null))
    }

    fn manifest_file(path: &Path) -> Result<Option<std::fs::File>, Failure> {
        Self::manifest_kind(path)?;
        match std::fs::File::open(path) {
            Ok(file) => Ok(Some(file)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(RuntimeAssets::failure(
                "client/manifest.json",
                &format!("cannot read manifest: {error}"),
            )),
        }
    }

    fn manifest_kind(path: &Path) -> Result<(), Failure> {
        let metadata = match std::fs::symlink_metadata(path) {
            Ok(metadata) => Some(metadata),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => {
                return Err(RuntimeAssets::failure(
                    "client/manifest.json",
                    &format!("cannot inspect manifest: {error}"),
                ));
            }
        };
        Self::require_manifest_file(metadata)
    }

    fn require_manifest_file(metadata: Option<std::fs::Metadata>) -> Result<(), Failure> {
        if metadata.is_some_and(|metadata| !metadata.file_type().is_file()) {
            return Err(RuntimeAssets::failure(
                "client/manifest.json",
                "manifest must be a regular non-symlinked file",
            ));
        }
        Ok(())
    }

    fn manifest_value(file: std::fs::File) -> Result<Value, Failure> {
        let mut bytes = Vec::new();
        file.take(1_048_577)
            .read_to_end(&mut bytes)
            .map_err(|error| {
                RuntimeAssets::failure(
                    "client/manifest.json",
                    &format!("cannot read manifest: {error}"),
                )
            })?;
        Self::manifest_length(&bytes)?;
        let source = std::str::from_utf8(&bytes).map_err(|error| {
            RuntimeAssets::failure(
                "client/manifest.json",
                &format!("manifest must be UTF-8: {error}"),
            )
        })?;
        serde_json::from_str(crate::without_bom(source)).map_err(|error| {
            RuntimeAssets::failure(
                "client/manifest.json",
                &format!("invalid manifest JSON: {error}"),
            )
        })
    }

    fn manifest_length(bytes: &[u8]) -> Result<(), Failure> {
        if bytes.len() > 1_048_576 {
            return Err(RuntimeAssets::failure(
                "client/manifest.json",
                "manifest exceeds 1 MiB",
            ));
        }
        Ok(())
    }

    fn selected(python: Option<&Value>) -> Result<Option<Self>, Failure> {
        python.map(Self::selected_profile).transpose()
    }

    fn selected_profile(python: &Value) -> Result<Self, Failure> {
        let settings = super::runtime_settings::RuntimeSettings::read(python)?;
        let fields = [
            settings.runtime == RuntimeAssets::RUNTIME,
            settings.profile == "offline",
        ];
        if fields.contains(&false) {
            return Err(RuntimeAssets::failure(
                "client/manifest.json",
                "tac.python must select runtime 'pyodide-314.0.7' and profile 'offline'",
            ));
        }
        Ok(Self)
    }

    fn limit(path: &str, bytes: &[u8]) -> Result<usize, Failure> {
        if path != RuntimeAssets::output_name(RuntimeAssets::WASM) {
            return Ok(Self::DEFAULT_ASSET_BYTES);
        }
        RuntimeAssets::validate(path, bytes)?;
        Ok(Self::WASM_ASSET_BYTES)
    }
}
