use std::ops::Range;

pub(super) struct DartTopLevel<'a> {
    code: &'a str,
    depth: i32,
    start: usize,
    ranges: Vec<Range<usize>>,
}

impl<'a> DartTopLevel<'a> {
    const OPENERS: [u8; 4] = *b"([{<";
    const CLOSERS: [u8; 4] = *b")]}>";
    const KINDS: [&'static [u8]; 4] = [b"{", b"([", b")]}", b";"];

    pub(super) fn ranges(code: &'a str) -> Vec<Range<usize>> {
        let mut scan = Self {
            code,
            depth: 0,
            start: 0,
            ranges: Vec::new(),
        };
        let length = code.len();
        std::iter::successors(Some(0), |at| (*at < length).then(|| scan.step(*at))).for_each(drop);
        scan.ranges.push(scan.start..length);
        scan.ranges
    }

    fn step(&mut self, at: usize) -> usize {
        let byte = self.code.as_bytes()[at];
        let kind = Self::KINDS
            .iter()
            .position(|bytes| bytes.contains(&byte))
            .unwrap_or(Self::KINDS.len());
        [
            Self::open,
            Self::deeper,
            Self::shallower,
            Self::semicolon,
            Self::next,
        ][kind](self, at)
    }

    fn open(&mut self, at: usize) -> usize {
        let block = [
            self.depth == 0,
            Self::block_header(&self.code[self.start..at]),
        ];
        [Self::deeper, Self::block][usize::from(block == [true, true])](self, at)
    }

    fn block(&mut self, at: usize) -> usize {
        let close = super::matching(self.code, at).unwrap_or(self.code.len() - 1);
        self.ranges.push(self.start..close + 1);
        self.start = close + 1;
        self.start
    }

    fn deeper(&mut self, at: usize) -> usize {
        self.depth += 1;
        at + 1
    }

    fn shallower(&mut self, at: usize) -> usize {
        self.depth -= 1;
        at + 1
    }

    fn semicolon(&mut self, at: usize) -> usize {
        [Self::next, Self::statement][usize::from(self.depth == 0)](self, at)
    }

    fn statement(&mut self, at: usize) -> usize {
        self.ranges.push(self.start..at + 1);
        self.start = at + 1;
        self.start
    }

    fn next(_: &mut Self, at: usize) -> usize {
        at + 1
    }

    fn block_header(header: &str) -> bool {
        !header
            .bytes()
            .scan(0_i32, |depth, byte| {
                *depth += Self::change(byte);
                Some((*depth, byte))
            })
            .any(|found| found == (0, b'='))
    }

    fn change(byte: u8) -> i32 {
        i32::from(Self::OPENERS.contains(&byte)) - i32::from(Self::CLOSERS.contains(&byte))
    }
}
