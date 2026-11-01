use super::{BodyBraceFinder, MethodHeader, SourceEdit};

pub(crate) struct BraceBody;

impl BraceBody {
    pub(crate) fn edits(language: &str, header: &MethodHeader<'_>) -> Vec<SourceEdit> {
        Self::open_brace(language, header)
            .map(|at| Self::hook_after(header, at))
            .into_iter()
            .collect()
    }

    pub(crate) fn hook_after(header: &MethodHeader<'_>, open: usize) -> SourceEdit {
        SourceEdit::insert(open + 1, header.hook.clone())
    }

    fn open_brace(language: &str, header: &MethodHeader<'_>) -> Option<usize> {
        let from = header.parameters_end().unwrap_or(header.start);
        BodyBraceFinder::find(language, header.masked, header.source, from)
    }
}
