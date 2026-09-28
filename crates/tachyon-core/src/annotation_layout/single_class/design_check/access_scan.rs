#![allow(clippy::needless_bitwise_bool)]

use std::ops::Range;

pub(super) struct AccessScan<'a> {
    text: &'a str,
    names: Vec<Range<usize>>,
}

impl<'a> AccessScan<'a> {
    const ACCESS_MARKS: [&'static str; 7] = [".", "?.", "!.", "!!.", "->", "?->", "::"];

    pub(super) fn read(text: &'a str) -> Self {
        Self {
            text,
            names: Self::name_ranges(text),
        }
    }

    pub(super) fn words(&self) -> Vec<&'a str> {
        self.names
            .iter()
            .map(|range| &self.text[range.clone()])
            .collect()
    }

    pub(super) fn accesses(&self) -> Vec<(&'a str, &'a str)> {
        self.names
            .windows(2)
            .filter_map(|pair| self.access(&pair[0], &pair[1]))
            .collect()
    }

    fn access(&self, before: &Range<usize>, name: &Range<usize>) -> Option<(&'a str, &'a str)> {
        let between = &self.text[before.end..name.start];
        let receiver = Self::receiver(between, &self.text[before.clone()]);
        Self::ends_with_mark(between).then(|| (receiver, &self.text[name.clone()]))
    }

    fn receiver(between: &str, before: &'a str) -> &'a str {
        let touching = Self::ACCESS_MARKS.contains(&between);
        [before, ""][usize::from(!touching)]
    }

    fn ends_with_mark(between: &str) -> bool {
        let trimmed = between.trim_end();
        Self::ACCESS_MARKS
            .iter()
            .any(|mark| trimmed.ends_with(mark))
    }

    fn name_ranges(text: &str) -> Vec<Range<usize>> {
        let mut ranges = Vec::new();
        let mut start: Option<usize> = None;
        for (at, letter) in text.char_indices() {
            start = Self::next_start(&mut ranges, start, at, letter);
        }
        ranges.extend(start.map(|first| first..text.len()));
        ranges
    }

    fn next_start(
        ranges: &mut Vec<Range<usize>>,
        start: Option<usize>,
        at: usize,
        letter: char,
    ) -> Option<usize> {
        let inside = letter.is_ascii_alphanumeric() | (letter == '_') | (letter == '$');
        let ended = start.filter(|_| !inside);
        ranges.extend(ended.map(|first| first..at));
        [None, start.or(Some(at))][usize::from(inside)]
    }
}
