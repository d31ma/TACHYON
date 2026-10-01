use super::runtime_assets::RuntimeAssets;
use crate::Failure;
use serde_json::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RuntimeProfile;

impl RuntimeProfile {
    pub(crate) const SCRIPT_BYTES: u64 = 512 * 1024;
    const DEFAULT_ASSET_BYTES: usize = 4 * 1024 * 1024;
    const WASM_ASSET_BYTES: usize = 10 * 1024 * 1024;

    pub(crate) fn read(manifest: &Value) -> Result<Option<Self>, Failure> {
        Self::selected(manifest.pointer("/tac/python"))
    }

    pub(crate) fn asset_limit(self, path: &str, bytes: &[u8]) -> Result<usize, Failure> {
        Self::limit(path, bytes)
    }

    fn selected(python: Option<&Value>) -> Result<Option<Self>, Failure> {
        let Some(python) = python else {
            return Ok(None);
        };
        if python.get("runtime").and_then(Value::as_str) != Some(RuntimeAssets::RUNTIME)
            || python.get("profile").and_then(Value::as_str) != Some("offline")
        {
            return Err(RuntimeAssets::failure(
                "client/manifest.json",
                "tac.python must select runtime 'pyodide-314.0.7' and profile 'offline'",
            ));
        }
        Ok(Some(Self))
    }

    fn limit(path: &str, bytes: &[u8]) -> Result<usize, Failure> {
        if path != RuntimeAssets::output_name(RuntimeAssets::WASM) {
            return Ok(Self::DEFAULT_ASSET_BYTES);
        }
        RuntimeAssets::validate(path, bytes)?;
        Ok(Self::WASM_ASSET_BYTES)
    }
}
