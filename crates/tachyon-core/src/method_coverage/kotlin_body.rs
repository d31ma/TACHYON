use super::{BraceBody, HookStatement, MethodHeader, SourceEdit};

pub(crate) struct KotlinBody;

impl KotlinBody {
    const BODY_STARTS: [char; 2] = ['{', '='];
    const EXPRESSION: u8 = b'=';

    pub(crate) fn edits(header: &MethodHeader<'_>) -> Vec<SourceEdit> {
        Self::body_start(header)
            .map(|at| Self::edit(header, at))
            .into_iter()
            .collect()
    }

    fn body_start(header: &MethodHeader<'_>) -> Option<usize> {
        let from = header.parameters_end()?;
        header.first_after(from, &Self::BODY_STARTS)
    }

    fn edit(header: &MethodHeader<'_>, at: usize) -> SourceEdit {
        if header.masked.as_bytes()[at] == Self::EXPRESSION {
            return SourceEdit::insert(at + 1, HookStatement::kotlin_expression(&header.key));
        }
        BraceBody::hook_after(header, at)
    }
}
