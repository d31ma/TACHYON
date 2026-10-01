use super::build_stage::BuildStage;
use super::component_script::ComponentScript;
use super::portable_path::PortablePath;
use super::typescript_compiler::TypeScriptCompiler;
use crate::Failure;
use std::fs;
use std::path::Path;

pub(super) struct ServiceScript;

impl ServiceScript {
    pub(super) async fn module(
        tools_root: &Path,
        snapshot_root: &Path,
        source: &Path,
    ) -> Result<Vec<u8>, Failure> {
        let portable = PortablePath::of(source.strip_prefix(snapshot_root).unwrap_or(source));
        let authored = ComponentScript::prepare_captured(
            snapshot_root,
            BuildStage::output_io(fs::read(source), source)?,
            &portable,
        )?;
        Self::compiled(tools_root, snapshot_root, source, &portable, authored).await
    }

    pub(super) fn has_extension(path: &Path, extension: &str) -> bool {
        path.extension().is_some_and(|value| value == extension)
    }

    async fn compiled(
        tools_root: &Path,
        snapshot_root: &Path,
        source: &Path,
        portable: &str,
        authored: Vec<u8>,
    ) -> Result<Vec<u8>, Failure> {
        if !Self::has_extension(source, "ts") {
            return Ok(authored);
        }
        TypeScriptCompiler::transpile(tools_root, snapshot_root, source, portable, &authored).await
    }
}
