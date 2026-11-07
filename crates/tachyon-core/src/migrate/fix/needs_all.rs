use super::edits::TextEdit;
use super::scan::SourceText;

pub(super) struct NeedsAllEdits;

impl NeedsAllEdits {
    const NAMES: [&'static str; 2] = ["NeedsAll(", "needs_all("];
    const MARKS: [char; 3] = ['@', '[', ':'];

    pub(super) fn plan(text: &SourceText<'_>) -> Vec<(usize, TextEdit)> {
        Self::NAMES
            .iter()
            .flat_map(|name| Self::name_edits(text, name))
            .collect()
    }

    fn name_edits(text: &SourceText<'_>, name: &str) -> Vec<(usize, TextEdit)> {
        text.source
            .match_indices(name)
            .filter_map(|(at, _)| Self::argument_edit(text, at, name).map(|edit| (at, edit)))
            .collect()
    }

    fn argument_edit(text: &SourceText<'_>, at: usize, name: &str) -> Option<TextEdit> {
        let before = text.source[..at].chars().next_back();
        let marked = before.is_some_and(|character| Self::MARKS.contains(&character));
        ([text.is_code(at), marked] == [true, true]).then_some(())?;
        let open = at + name.len() - 1;
        let close = Self::closing(text, open)?;
        let has_arguments = !text.source[open + 1..close].trim().is_empty();
        has_arguments.then(|| TextEdit::delete(open + 1, close))
    }

    fn closing(text: &SourceText<'_>, open: usize) -> Option<usize> {
        let mut depth = 0_usize;
        for at in open..text.len() {
            if !text.is_code(at) {
                continue;
            }
            match text.bytes[at] {
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(at);
                    }
                }
                b'\n' => return None,
                _ => {}
            }
        }
        None
    }
}
