use super::{BodyBrace, MethodHeader, SourceEdit};

pub(crate) struct BraceBody;

impl BraceBody {
    pub(crate) fn edits(header: &MethodHeader<'_>) -> Vec<SourceEdit> {
        Self::open_brace(header)
            .map(|at| Self::hook_after(header, at))
            .into_iter()
            .collect()
    }

    pub(crate) fn hook_after(header: &MethodHeader<'_>, open: usize) -> SourceEdit {
        SourceEdit::insert(open + 1, header.hook.clone())
    }

    fn open_brace(header: &MethodHeader<'_>) -> Option<usize> {
        let from = header.parameters_end().unwrap_or(header.start);
        BodyBrace::find(header.masked, from)
    }
}
