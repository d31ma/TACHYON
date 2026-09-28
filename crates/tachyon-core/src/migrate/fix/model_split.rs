use super::edits::TextEdit;
use super::model_text::ModelText;
use crate::annotation_layout::OneModelCheck;
use crate::model_layout::{ModelBlock, ModelBlocks};
use std::ffi::OsStr;
use std::path::Path;

pub(super) struct ModelSplit<'a> {
    pub(super) relative: &'a str,
    source: &'a str,
    code: String,
    blocks: Vec<ModelBlock>,
    kept: usize,
    pub(super) extension: &'a str,
    pub(super) folder: String,
    pub(super) stem: &'a str,
}

impl<'a> ModelSplit<'a> {
    const BLANK_LINE: &'static str = "\n\n";
    const JAVASCRIPT: &'static str = "js";
    const EMPTY_EXPORT: &'static str = "export {}\n";

    pub(super) fn read(relative: &'a str, source: &'a str) -> Option<Self> {
        let path = Path::new(relative);
        let blocks = OneModelCheck::applies(path)
            .then(|| ModelBlocks::read(path, source).ok())
            .flatten()?;
        let code = crate::annotation_layout::masked_source(path, source).ok()?;
        let split = Self {
            relative,
            source,
            code,
            kept: Self::kept_index(&blocks, Self::text(path.file_stem())),
            blocks,
            extension: Self::text(path.extension()),
            folder: Self::folder(path),
            stem: Self::text(path.file_stem()),
        };
        split.has_several_models().then_some(split)
    }

    pub(super) fn moved(&self) -> Vec<&ModelBlock> {
        self.blocks
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != self.kept)
            .map(|(_, block)| block)
            .collect()
    }

    pub(super) fn moved_names(&self) -> Vec<&str> {
        self.moved()
            .into_iter()
            .map(|block| block.name.as_str())
            .collect()
    }

    pub(super) fn source_text(&self) -> &'a str {
        self.source
    }

    pub(super) fn target(&self, name: &str) -> String {
        format!("{}{name}.{}", self.folder, self.extension)
    }

    pub(super) fn new_file(&self, block: &ModelBlock) -> String {
        let imports = self.imports_for(block);
        let head = [self.preamble(), imports.trim_end()]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<&str>>()
            .join("\n");
        let gap = self.gap_after(&head);
        let ending = self.module_ending();
        format!(
            "{head}{gap}{}{ending}",
            Self::ended(block.text(self.source))
        )
    }

    pub(super) fn original_edits(&self) -> Vec<TextEdit> {
        let mut edits = self.removals();
        edits.extend(self.kept_imports());
        edits
    }

    fn module_ending(&self) -> &'static str {
        if self.extension != Self::JAVASCRIPT {
            return "";
        }
        Self::EMPTY_EXPORT
    }

    fn has_several_models(&self) -> bool {
        self.blocks.len() > 1
    }

    fn kept_index(blocks: &[ModelBlock], stem: &str) -> usize {
        blocks
            .iter()
            .position(|block| block.name == stem)
            .unwrap_or(0)
    }

    fn text(value: Option<&'a OsStr>) -> &'a str {
        value.and_then(OsStr::to_str).unwrap_or("")
    }

    fn folder(path: &Path) -> String {
        let parent = path.parent().unwrap_or(Path::new(""));
        format!("{}/", parent.to_string_lossy().replace('\\', "/"))
    }

    fn first_start(&self) -> usize {
        self.blocks.first().map_or(0, |block| block.start)
    }

    fn preamble(&self) -> &str {
        self.source[..self.first_start()].trim_end()
    }

    fn gap_after(&self, head: &str) -> &str {
        if head.is_empty() {
            return "";
        }
        let preamble = self.preamble();
        let gap = &self.source[preamble.len()..self.first_start()];
        [gap, Self::BLANK_LINE][usize::from(preamble.is_empty())]
    }

    fn ended(text: &str) -> String {
        format!("{}\n", text.trim_end())
    }

    fn imports_for(&self, block: &ModelBlock) -> String {
        let others: Vec<&str> = self
            .blocks
            .iter()
            .filter(|other| other.name != block.name)
            .map(|other| other.name.as_str())
            .collect();
        let used = ModelText::mentioned(block.text(&self.code), &others);
        ModelText::import_lines(self.extension, &used)
    }

    fn kept_imports(&self) -> Option<TextEdit> {
        let kept = self.blocks.get(self.kept)?;
        let used = ModelText::mentioned(kept.text(&self.code), &self.moved_names());
        let imports = ModelText::import_lines(self.extension, &used);
        (!imports.is_empty()).then(|| self.import_edit(&imports, kept))
    }

    fn import_edit(&self, imports: &str, kept: &ModelBlock) -> TextEdit {
        let preamble_end = self.preamble().len();
        if preamble_end == 0 {
            return TextEdit::insert(kept.start, &format!("{imports}\n"));
        }
        TextEdit::insert(preamble_end, &format!("\n{}", imports.trim_end()))
    }

    fn removals(&self) -> Vec<TextEdit> {
        let ranges: Vec<(usize, usize)> = self
            .moved()
            .into_iter()
            .map(|block| (block.start, self.after_blank_lines(block.end)))
            .collect();
        ModelText::merged(&ranges)
            .into_iter()
            .map(|(start, end)| self.removal(start, end))
            .collect()
    }

    fn removal(&self, start: usize, end: usize) -> TextEdit {
        let reaches_end = end == self.source.len();
        let before = self.source[..start].trim_end().len();
        let start = [start, (before + 1).min(start)][usize::from(reaches_end)];
        TextEdit::delete(start, end)
    }

    fn after_blank_lines(&self, end: usize) -> usize {
        let blank: usize = self.source[end..]
            .split_inclusive('\n')
            .take_while(|line| line.trim().is_empty())
            .map(str::len)
            .sum();
        end + blank
    }
}
