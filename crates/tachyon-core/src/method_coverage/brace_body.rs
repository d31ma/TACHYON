use super::{MethodHeader, SourceEdit};

pub(crate) struct BraceBody;

impl BraceBody {
    const OPEN: char = '{';
    const CLOSE: char = '}';

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
        let line = header.line().trim_end();
        if line.ends_with(Self::OPEN) {
            return line.rfind(Self::OPEN).map(|at| header.start + at);
        }
        if let Some(at) = Self::last_group_open(line) {
            return Some(header.start + at);
        }
        let from = header.parameters_end().unwrap_or(header.start);
        header.first_after(from, &[Self::OPEN])
    }

    fn last_group_open(line: &str) -> Option<usize> {
        let balanced = line.matches(Self::OPEN).count() == line.matches(Self::CLOSE).count();
        if [balanced, line.ends_with(Self::CLOSE)] != [true, true] {
            return None;
        }
        line.char_indices()
            .rev()
            .scan(0_i64, |depth, (at, character)| {
                *depth += match character {
                    Self::CLOSE => 1,
                    Self::OPEN => -1,
                    _ => 0,
                };
                Some((at, character, *depth))
            })
            .find(|(_, character, depth)| [*character == Self::OPEN, *depth == 0] == [true, true])
            .map(|(at, _, _)| at)
    }
}
