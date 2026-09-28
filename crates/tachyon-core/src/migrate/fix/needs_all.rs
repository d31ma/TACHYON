use super::edits::TextEdit;
use super::scan::SourceText;

pub(super) struct NeedsAllEdits;

impl NeedsAllEdits {
    const NAMES: [&'static str; 2] = ["NeedsAll(", "needs_all("];

    pub(super) fn plan(text: &SourceText<'_>) -> Vec<(usize, TextEdit)> {
        let mut edits = Vec::new();
        for name in Self::NAMES {
            for (at, _) in text.source.match_indices(name) {
                let before = text.source[..at].chars().next_back();
                if !text.is_code(at) || !matches!(before, Some('@' | '[' | ':')) {
                    continue;
                }
                let open = at + name.len() - 1;
                let Some(close) = Self::closing(text, open) else {
                    continue;
                };
                if !text.source[open + 1..close].trim().is_empty() {
                    edits.push((at, TextEdit::delete(open + 1, close)));
                }
            }
        }
        edits
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
