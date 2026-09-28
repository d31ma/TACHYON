use super::FileNameCheck;
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use crate::model_layout::{ModelBlock, ModelBlocks};
use std::ffi::OsStr;
use std::path::Path;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;

pub(crate) struct OneModelCheck<'a> {
    path: &'a Path,
    blocks: Vec<ModelBlock>,
}

impl<'a> OneModelCheck<'a> {
    pub(crate) const ENFORCED: bool = true;
    const MODEL_FOLDERS: [&'static str; 2] = ["client/models", "server/models"];
    const RULE: &'static str =
        "A models file holds one model, and the file is named after it (ADR 0067).";

    pub(crate) fn check(path: &'a Path, source: &str) -> Result<(), Failure> {
        let file = Self::read(path, source)?;
        file.require_one_model()
    }

    pub(crate) fn applies(path: &Path) -> bool {
        let checks = [Self::ENFORCED, Self::in_models_folder(path)];
        !checks.contains(&false)
    }

    fn read(path: &'a Path, source: &str) -> Result<Self, Failure> {
        if !Self::applies(path) {
            return Ok(Self {
                path,
                blocks: Vec::new(),
            });
        }
        let blocks = ModelBlocks::read(path, source)?;
        Ok(Self { path, blocks })
    }

    fn in_models_folder(path: &Path) -> bool {
        Self::MODEL_FOLDERS
            .iter()
            .any(|folder| path.starts_with(folder))
    }

    fn extension(path: &Path) -> &str {
        path.extension().and_then(OsStr::to_str).unwrap_or("")
    }

    fn require_one_model(&self) -> Result<(), Failure> {
        let Some(second) = self.blocks.get(1) else {
            return Ok(());
        };
        Err(self.failure(second))
    }

    fn failure(&self, second: &ModelBlock) -> Failure {
        let file = self.path.to_string_lossy().replace('\\', "/");
        let name = self.path.file_name().and_then(OsStr::to_str).unwrap_or("");
        let extension = Self::extension(self.path);
        let names: Vec<&str> = self
            .blocks
            .iter()
            .map(|block| block.name.as_str())
            .collect();
        let message = format!(
            "The models file '{name}' declares {} models ({}); a models file declares one model",
            names.len(),
            names.join(", ")
        );
        let help = format!(
            "Move {} into its own file '{}.{extension}' in the same folder, and keep one model in '{name}'. {} `ty migrate fix --write` splits the file for you.",
            second.name,
            second.name,
            Self::RULE
        );
        Failure::one(diagnostic(
            FileNameCheck::CODE,
            message,
            Some(help),
            source_span(&file, second.start, second.start + 1),
        ))
    }
}
