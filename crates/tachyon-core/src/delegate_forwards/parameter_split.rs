use std::ops::Range;

pub(super) struct ParameterSplit;

impl ParameterSplit {
    const OPENERS: [char; 4] = ['(', '[', '<', '{'];
    const CLOSERS: [char; 4] = [')', ']', '>', '}'];
    const ARROW_STARTS: [char; 2] = ['-', '='];

    pub(super) fn matching_close(masked: &str, open: usize) -> Option<usize> {
        let mut depth = 0_usize;
        masked[open..]
            .char_indices()
            .find(|(_, letter)| {
                depth = (depth + usize::from(*letter == '('))
                    .saturating_sub(usize::from(*letter == ')'));
                [*letter == ')', depth == 0] == [true, true]
            })
            .map(|(offset, _)| open + offset)
    }

    pub(super) fn parts(source: &str, masked: &str, inside: Range<usize>) -> Vec<String> {
        let commas = Self::top_level_commas(&masked[inside.clone()]);
        let starts = std::iter::once(0).chain(commas.iter().map(|comma| comma + 1));
        let ends = commas.iter().copied().chain(std::iter::once(inside.len()));
        starts
            .zip(ends)
            .map(|(start, end)| source[inside.start + start..inside.start + end].trim())
            .filter(|part| !part.is_empty())
            .map(str::to_owned)
            .collect()
    }

    pub(super) fn top_level_commas(text: &str) -> Vec<usize> {
        let mut depth = 0_usize;
        let mut previous = ' ';
        text.char_indices()
            .filter(|(_, letter)| {
                let arrow = [*letter == '>', Self::ARROW_STARTS.contains(&previous)] == [true, true];
                let opened = usize::from(Self::OPENERS.contains(letter));
                let closed = usize::from([Self::CLOSERS.contains(letter), !arrow] == [true, true]);
                let comma = [*letter == ',', depth == 0] == [true, true];
                depth = (depth + opened).saturating_sub(closed);
                previous = *letter;
                comma
            })
            .map(|(offset, _)| offset)
            .collect()
    }
}
