use crate::Failure;
use braced_blocks::BracedBlocks;
use js_doc_blocks::JsDocBlocks;
use leading_lines::LeadingLines;
use line_blocks::LineBlocks;
use std::ffi::OsStr;
use std::path::Path;
pub(super) use string_fill::StringFill;

mod braced_blocks;
mod js_doc_blocks;
mod leading_lines;
mod line_blocks;
mod model_block;
mod string_fill;
mod type_name;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;

pub(crate) use model_block::ModelBlock;

pub(crate) struct ModelBlocks;

impl ModelBlocks {
    const MODEL_FOLDERS: [&'static str; 2] = ["client/models", "server/models"];

    pub(crate) fn read(path: &Path, source: &str) -> Result<Vec<ModelBlock>, Failure> {
        Self::captured(path, source, Self::model_path(path))
    }

    pub(crate) fn schema_declarations(
        path: &Path,
        source: &str,
    ) -> Result<Vec<ModelBlock>, Failure> {
        Self::captured(path, source, true)
    }

    fn model_path(path: &Path) -> bool {
        Self::MODEL_FOLDERS
            .iter()
            .any(|folder| path.starts_with(folder))
    }

    fn captured(path: &Path, source: &str, classes: bool) -> Result<Vec<ModelBlock>, Failure> {
        let masked = crate::annotation_layout::masked_source(path, source)?;
        let extension = path.extension().and_then(OsStr::to_str).unwrap_or("");
        let code = StringFill::apply(extension, source, &masked);
        let found = Self::found(extension, source, &code, classes);
        Ok(Self::widened(&found, &LeadingLines::new(source, &code)))
    }

    pub(crate) fn names(path: &Path, source: &str) -> Result<Vec<String>, Failure> {
        let blocks = Self::read(path, source)?;
        Ok(blocks.into_iter().map(|block| block.name).collect())
    }

    fn found(extension: &str, source: &str, code: &str, classes: bool) -> Vec<ModelBlock> {
        match (extension, classes) {
            ("fs", _) => LineBlocks::fsharp(code),
            ("py", _) => LineBlocks::python(code),
            ("rb", _) => LineBlocks::ruby(code),
            ("js", false) => JsDocBlocks::read(source),
            ("js", true) => Self::javascript(source, code),
            _ => BracedBlocks::read(extension, code),
        }
    }

    fn javascript(source: &str, code: &str) -> Vec<ModelBlock> {
        let mut blocks = JsDocBlocks::read(source);
        blocks.extend(BracedBlocks::read("js", code));
        blocks.sort_by_key(|block| block.start);
        blocks
    }

    fn widened(found: &[ModelBlock], lines: &LeadingLines<'_>) -> Vec<ModelBlock> {
        let floors = std::iter::once(0).chain(found.iter().map(|block| block.end));
        found
            .iter()
            .zip(floors)
            .map(|(block, floor)| lines.widen(block, floor))
            .collect()
    }
}
