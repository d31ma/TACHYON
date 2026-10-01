use super::build_stage::BuildStage;
use super::component_script::ComponentScript;
use super::portable_path::PortablePath;
use super::typescript_compiler::TypeScriptCompiler;
use crate::Failure;
use std::fs;
use std::path::{Path, PathBuf};

pub(super) struct ServiceScript {
    pub(super) bytes: Vec<u8>,
    pub(super) behavior_runtime: bool,
}

impl ServiceScript {
    const RUNTIME: &'static str = include_str!("../behavior/runtime.js");
    const RUNTIME_PATH: &'static str = ".tachyon/behavior-runtime.js";
    const RUNTIME_IMPORT: &'static [u8] =
        b"import { __tachyon_create_behaviors__ } from '../behavior-runtime.js';\n";

    pub(super) async fn module(
        tools_root: &Path,
        snapshot_root: &Path,
        source: &Path,
    ) -> Result<Self, Failure> {
        let portable = PortablePath::of(source.strip_prefix(snapshot_root).unwrap_or(source));
        let authored = ComponentScript::prepare_captured(
            snapshot_root,
            BuildStage::output_io(fs::read(source), source)?,
            &portable,
        )?;
        let mut module = Self::separated(authored);
        module.bytes =
            Self::compiled(tools_root, snapshot_root, source, &portable, module.bytes).await?;
        Ok(module)
    }

    fn separated(mut bytes: Vec<u8>) -> Self {
        let offset = String::from_utf8_lossy(&bytes).rfind(Self::RUNTIME);
        if let Some(offset) = offset {
            bytes.drain(offset..offset + Self::RUNTIME.len());
        }
        Self {
            bytes,
            behavior_runtime: offset.is_some(),
        }
    }

    pub(super) fn linked(bytes: Vec<u8>, shared: bool) -> Vec<u8> {
        Self::linked_bytes(bytes, shared)
    }

    fn linked_bytes(bytes: Vec<u8>, shared: bool) -> Vec<u8> {
        if !shared {
            return bytes;
        }
        let mut output = Self::RUNTIME_IMPORT.to_vec();
        output.extend(bytes);
        output
    }

    pub(super) fn runtime(enabled: bool) -> Option<(PathBuf, Vec<u8>)> {
        enabled.then(|| {
            let mut source = b"export ".to_vec();
            source.extend_from_slice(Self::RUNTIME.as_bytes());
            (PathBuf::from(Self::RUNTIME_PATH), source)
        })
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
