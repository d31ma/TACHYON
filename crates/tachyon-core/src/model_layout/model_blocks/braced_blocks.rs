use super::super::piece::Piece;
use super::model_block::ModelBlock;
use super::type_name::TypeName;

pub(super) struct BracedBlocks;

impl BracedBlocks {
    const LINE_LANGUAGES: [&'static str; 4] = ["ts", "kt", "swift", "go"];
    const PHP_START: &'static str = "<?php";

    pub(super) fn read(extension: &str, code: &str) -> Vec<ModelBlock> {
        let whole = Piece::new(0, code).trim();
        let code = whole.after(Self::PHP_START).unwrap_or(whole);
        Self::parts(extension, code)
            .into_iter()
            .filter_map(Self::block)
            .collect()
    }

    fn parts<'a>(extension: &str, code: Piece<'a>) -> Vec<Piece<'a>> {
        if Self::LINE_LANGUAGES.contains(&extension) {
            return code.statements();
        }
        code.items()
    }

    fn block(part: Piece<'_>) -> Option<ModelBlock> {
        let name = TypeName::read(part)?;
        Some(ModelBlock::new(name, part.at, part.at + part.text.len()))
    }
}
