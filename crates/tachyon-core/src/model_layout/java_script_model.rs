use super::piece::Piece;

pub(super) struct JavaScriptModel;

impl JavaScriptModel {
    const EXPORT_WORD: &'static str = "export";
    const EMPTY_GROUP: &'static str = "{}";

    pub(super) fn first_refused(code: Piece<'_>) -> Option<usize> {
        let refused = code.statements().into_iter().find(Self::is_code)?;
        Some(refused.at)
    }

    fn is_code(statement: &Piece<'_>) -> bool {
        !Self::is_empty_export(*statement)
    }

    fn is_empty_export(statement: Piece<'_>) -> bool {
        let Some(rest) = statement.after_word(Self::EXPORT_WORD) else {
            return false;
        };
        rest.text.replace(char::is_whitespace, "") == Self::EMPTY_GROUP
    }
}
