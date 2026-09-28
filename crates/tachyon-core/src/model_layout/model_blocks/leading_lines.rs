use super::model_block::ModelBlock;

pub(super) struct LeadingLines<'a> {
    source: &'a str,
    code: &'a str,
}

impl<'a> LeadingLines<'a> {
    const ANNOTATION_STARTS: [char; 3] = ['@', '#', '['];

    pub(super) fn new(source: &'a str, code: &'a str) -> Self {
        Self { source, code }
    }

    pub(super) fn widen(&self, block: &ModelBlock, floor: usize) -> ModelBlock {
        let start = self.first_attached_line(self.line_start(block.start), floor);
        let end = self.line_end(block.end);
        ModelBlock::new(&block.name, start, end)
    }

    fn line_start(&self, at: usize) -> usize {
        let start = Self::start_of_line(self.code, at);
        let alone = self.code[start..at].trim().is_empty();
        [at, start][usize::from(alone)]
    }

    fn line_end(&self, end: usize) -> usize {
        let stop = self.code[end..]
            .find('\n')
            .map_or(self.code.len(), |index| end + index + 1);
        let rest_is_blank = self.code[end..stop].trim().is_empty();
        [end, stop][usize::from(rest_is_blank)]
    }

    fn first_attached_line(&self, mut start: usize, floor: usize) -> usize {
        while let Some(previous) = self
            .attached_line_before(start)
            .filter(|previous| *previous >= floor)
        {
            start = previous;
        }
        start
    }

    fn attached_line_before(&self, start: usize) -> Option<usize> {
        let newline = start.checked_sub(1)?;
        let previous = Self::start_of_line(self.code, newline);
        self.is_attached(previous, newline).then_some(previous)
    }

    fn is_attached(&self, from: usize, to: usize) -> bool {
        let written = !self.source[from..to].trim().is_empty();
        let code = self.code[from..to].trim();
        let checks = [
            written,
            [code.is_empty(), code.starts_with(Self::ANNOTATION_STARTS)].contains(&true),
        ];
        !checks.contains(&false)
    }

    fn start_of_line(text: &str, at: usize) -> usize {
        text[..at].rfind('\n').map_or(0, |index| index + 1)
    }
}
