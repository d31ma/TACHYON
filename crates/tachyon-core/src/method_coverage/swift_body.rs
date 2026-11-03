use super::{BraceBody, MethodHeader, SourceEdit};

pub(crate) struct SwiftBody;

impl SwiftBody {
    const OPEN: char = '{';
    const RETURNS_VALUE: &'static str = "->";
    const RETURN: &'static str = "return ";
    const STATEMENT_WORDS: [&'static str; 14] = [
        "return", "let", "var", "if", "guard", "for", "while", "switch", "throw", "defer", "do",
        "repeat", "try", "await",
    ];

    pub(crate) fn edits(header: &MethodHeader<'_>) -> Vec<SourceEdit> {
        let open = header.first_after(
            header.parameters_end().unwrap_or(header.start),
            &[Self::OPEN],
        );
        open.map(|open| Self::edits_at(header, open))
            .unwrap_or_default()
    }

    fn edits_at(header: &MethodHeader<'_>, open: usize) -> Vec<SourceEdit> {
        let mut edits = vec![BraceBody::hook_after(header, open)];
        edits.extend(Self::implicit_return(header, open));
        edits
    }

    fn implicit_return(header: &MethodHeader<'_>, open: usize) -> Option<SourceEdit> {
        let close = MethodHeader::closing(header.masked, open)? - 1;
        let body = &header.source[open + 1..close];
        let expression = body.trim();
        let returns_value = header.masked[header.start..open].contains(Self::RETURNS_VALUE);
        let single_expression =
            !expression.contains(['\n', ';']) && !Self::starts_statement(expression);
        let at = open + 1 + body.len() - body.trim_start().len();
        [returns_value, single_expression]
            .iter()
            .all(|holds| *holds)
            .then(|| SourceEdit::insert(at, String::from(Self::RETURN)))
    }

    fn starts_statement(expression: &str) -> bool {
        let first_word = expression
            .split(|character: char| !character.is_alphanumeric())
            .next()
            .unwrap_or_default();
        Self::STATEMENT_WORDS.contains(&first_word)
    }
}
