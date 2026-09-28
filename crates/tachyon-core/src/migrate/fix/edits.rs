#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct TextEdit {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) replacement: String,
}

impl TextEdit {
    pub(super) fn delete(start: usize, end: usize) -> Self {
        Self::replace(start, end, "")
    }

    pub(super) fn insert(at: usize, text: &str) -> Self {
        Self::replace(at, at, text)
    }

    pub(super) fn replace(start: usize, end: usize, replacement: &str) -> Self {
        Self {
            start,
            end,
            replacement: String::from(replacement),
        }
    }

    pub(super) fn sorted(edits: &[Self]) -> Vec<Self> {
        let mut sorted = edits.to_vec();
        sorted.sort_by_key(|edit| (edit.start, edit.end));
        sorted
    }

    pub(super) fn are_separate(edits: &[Self]) -> bool {
        let mut previous_end = 0;
        for edit in Self::sorted(edits) {
            if edit.start < previous_end {
                return false;
            }
            previous_end = edit.end;
        }
        true
    }

    pub(super) fn apply(source: &str, edits: &[Self]) -> String {
        let mut text = String::with_capacity(source.len());
        let mut cursor = 0;
        for edit in Self::sorted(edits) {
            text.push_str(&source[cursor..edit.start]);
            text.push_str(&edit.replacement);
            cursor = edit.end;
        }
        text.push_str(&source[cursor..]);
        text
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Hunk {
    pub(super) line: usize,
    pub(super) removed: Vec<String>,
    pub(super) added: Vec<String>,
}

impl Hunk {
    pub(super) fn from_edits(source: &str, edits: &[TextEdit]) -> Vec<Self> {
        let mut hunks = Vec::new();
        let mut group: Vec<TextEdit> = Vec::new();
        let mut group_start = 0;
        let mut group_end = 0;
        for edit in TextEdit::sorted(edits) {
            let (start, end) = Self::lines_around(source, &edit);
            if !group.is_empty() && start >= group_end {
                hunks.push(Self::from_group(source, group_start, group_end, &group));
                group.clear();
            }
            if group.is_empty() {
                group_start = start;
            }
            group_end = group_end.max(end);
            group.push(edit);
        }
        if !group.is_empty() {
            hunks.push(Self::from_group(source, group_start, group_end, &group));
        }
        hunks
    }

    fn lines_around(source: &str, edit: &TextEdit) -> (usize, usize) {
        let start = source[..edit.start].rfind('\n').map_or(0, |at| at + 1);
        let ends_on_line_start = edit.end > edit.start && source[..edit.end].ends_with('\n');
        if ends_on_line_start {
            return (start, edit.end);
        }
        let end = source[edit.end..]
            .find('\n')
            .map_or(source.len(), |at| edit.end + at + 1);
        (start, end)
    }

    fn from_group(source: &str, start: usize, end: usize, group: &[TextEdit]) -> Self {
        let mut shifted = Vec::new();
        for edit in group {
            let replacement = edit.replacement.as_str();
            shifted.push(TextEdit::replace(
                edit.start - start,
                edit.end - start,
                replacement,
            ));
        }
        let before = &source[start..end];
        let after = TextEdit::apply(before, &shifted);
        Self {
            line: source[..start].matches('\n').count() + 1,
            removed: before.lines().map(String::from).collect(),
            added: after.lines().map(String::from).collect(),
        }
    }
}
