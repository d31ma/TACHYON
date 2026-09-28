use super::{MethodHeader, SourceEdit};

pub(crate) struct BraceBody;

impl BraceBody {
    const OPEN: char = '{';

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
        let line = header.line();
        if line.trim_end().ends_with(Self::OPEN) {
            return line.rfind(Self::OPEN).map(|at| header.start + at);
        }
        let from = header.parameters_end().unwrap_or(header.start);
        header.first_after(from, &[Self::OPEN])
    }
}
