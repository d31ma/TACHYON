use super::build_stage::BuildStage;
use super::component_script::ComponentScript;
use super::portable_path::PortablePath;
use super::shared_assets::SharedAssets;
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
        let bytes = Self::compiled(tools_root, snapshot_root, source, &portable, authored).await?;
        let bytes = Self::rewrite_client_shared_imports(snapshot_root, source, bytes);
        Ok(Self::make_shared_imports_relative(bytes))
    }

    pub(super) fn has_extension(path: &Path, extension: &str) -> bool {
        path.extension().is_some_and(|value| value == extension)
    }

    pub(super) fn rewrite_client_shared_imports(
        project_root: &Path,
        source: &Path,
        bytes: Vec<u8>,
    ) -> Vec<u8> {
        let depth = Self::client_depth(project_root, source);
        Self::rewrite_at_depth(depth, bytes)
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

    fn make_shared_imports_relative(bytes: Vec<u8>) -> Vec<u8> {
        SharedAssets::edit_text(bytes, |module| {
            module
                .replace("'/shared/", "'../../shared/")
                .replace("\"/shared/", "\"../../shared/")
        })
    }

    fn client_depth(project_root: &Path, source: &Path) -> Option<usize> {
        let parent = source.parent()?;
        let relative_parent = parent.strip_prefix(project_root.join("client")).ok()?;
        let depth = relative_parent.components().count();
        (depth > 0).then_some(depth)
    }

    fn rewrite_at_depth(depth: Option<usize>, bytes: Vec<u8>) -> Vec<u8> {
        match depth {
            Some(depth) => SharedAssets::edit_text(bytes, |module| {
                Self::absolute_shared_imports(module, depth)
            }),
            None => bytes,
        }
    }

    fn absolute_shared_imports(module: String, depth: usize) -> String {
        let authored = format!("{}shared/", "../".repeat(depth));
        let mut module = module;
        for quote in ['\'', '"'] {
            module = module.replace(&format!("{quote}{authored}"), &format!("{quote}/shared/"));
        }
        module
    }
}
