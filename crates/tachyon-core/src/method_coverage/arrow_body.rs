use super::{BraceBody, MethodHeader, SourceEdit};

pub(crate) struct ArrowBody;

impl ArrowBody {
    const BODY_STARTS: [char; 2] = ['{', '='];
    const ARROW: u8 = b'=';
    const ARROW_LENGTH: usize = 2;
    const STATEMENT_END: u8 = b';';
    const NO_VALUE: &'static str = "void";
    const RETURN: &'static str = " return";
    const THROW: &'static str = "throw ";

    pub(crate) fn edits(header: &MethodHeader<'_>) -> Vec<SourceEdit> {
        Self::body_start(header)
            .map(|at| Self::edits_at(header, at))
            .unwrap_or_default()
    }

    fn body_start(header: &MethodHeader<'_>) -> Option<usize> {
        let from = header.parameters_end()?;
        header.first_after(from, &Self::BODY_STARTS)
    }

    fn edits_at(header: &MethodHeader<'_>, at: usize) -> Vec<SourceEdit> {
        if header.masked.as_bytes()[at] == Self::ARROW {
            return Self::arrow_edits(header, at).unwrap_or_default();
        }
        vec![BraceBody::hook_after(header, at)]
    }

    fn arrow_edits(header: &MethodHeader<'_>, arrow: usize) -> Option<Vec<SourceEdit>> {
        let expression = arrow + Self::ARROW_LENGTH;
        let end = expression
            + MethodHeader::find_at_depth_zero(&header.masked[expression..], Self::STATEMENT_END)?;
        let opening = format!("{{{}{}", header.hook, Self::return_word(header, expression));
        Some(vec![
            SourceEdit::replace(arrow..expression, opening),
            SourceEdit::insert(end + 1, String::from(" }")),
        ])
    }

    fn return_word(header: &MethodHeader<'_>, expression: usize) -> &'static str {
        let signature = &header.line()[..header.name_end().unwrap_or(header.start) - header.start];
        let returns_void = signature
            .split_whitespace()
            .any(|word| word == Self::NO_VALUE);
        let throws = header.masked[expression..]
            .trim_start()
            .starts_with(Self::THROW);
        if [returns_void, throws].contains(&true) {
            return "";
        }
        Self::RETURN
    }
}
