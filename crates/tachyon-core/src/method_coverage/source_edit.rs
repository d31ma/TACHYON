use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceEdit {
    pub(crate) range: Range<usize>,
    pub(crate) text: String,
}

impl SourceEdit {
    pub(crate) fn insert(at: usize, text: String) -> Self {
        Self {
            range: at..at,
            text,
        }
    }

    pub(crate) fn replace(range: Range<usize>, text: String) -> Self {
        Self { range, text }
    }

    pub(crate) fn apply_all(source: &str, mut edits: Vec<Self>) -> String {
        edits.sort_by_key(|edit| std::cmp::Reverse(edit.range.start));
        let mut output = source.to_owned();
        Self::apply_each(&mut output, &edits);
        output
    }

    fn apply_each(output: &mut String, edits: &[Self]) {
        for edit in edits {
            output.replace_range(edit.range.clone(), &edit.text);
        }
    }
}
