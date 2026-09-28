use super::super::piece::Piece;
use super::model_block::ModelBlock;

pub(super) struct JsDocBlocks;

impl JsDocBlocks {
    const OPEN: &'static str = "/**";
    const CLOSE: &'static str = "*/";
    const TYPEDEF: &'static str = "@typedef";

    pub(super) fn read(source: &str) -> Vec<ModelBlock> {
        Self::comments(source)
            .into_iter()
            .filter_map(Self::block)
            .collect()
    }

    fn comments(source: &str) -> Vec<Piece<'_>> {
        source
            .match_indices(Self::OPEN)
            .filter_map(|(start, _)| Self::comment(source, start))
            .collect()
    }

    fn comment(source: &str, start: usize) -> Option<Piece<'_>> {
        let length = source[start..].find(Self::CLOSE)? + Self::CLOSE.len();
        Some(Piece::new(start, &source[start..start + length]))
    }

    fn block(comment: Piece<'_>) -> Option<ModelBlock> {
        let at = comment.text.find(Self::TYPEDEF)? + Self::TYPEDEF.len();
        let rest = comment.from(at).trim();
        let name = Self::name(Self::after_type(rest))?;
        Some(ModelBlock::new(
            name,
            comment.at,
            comment.at + comment.text.len(),
        ))
    }

    fn after_type(rest: Piece<'_>) -> Piece<'_> {
        if !rest.starts_with("{") {
            return rest;
        }
        rest.after_group().unwrap_or(rest.from(rest.text.len()))
    }

    fn name(rest: Piece<'_>) -> Option<&str> {
        Some(rest.first_word()).filter(|name| !name.is_empty())
    }
}
