use crate::duplicate::Duplicate;
use crate::duplicate_finder::DuplicateFinder;
use crate::place::Place;
use crate::repeat::Repeat;
use crate::token_stream::TokenStream;
use crate::window_hashes::WindowHashes;
use std::collections::HashMap;

pub(crate) struct LanguageScan<'s> {
    stream: &'s TokenStream<'s>,
    hashes: Vec<u64>,
    starts: HashMap<u64, Vec<usize>>,
}

impl<'s> LanguageScan<'s> {
    const WIDTH: usize = DuplicateFinder::MIN_TOKENS;
    const WORD_SHARE: usize = 10;

    pub(crate) fn new(stream: &'s TokenStream<'s>) -> Self {
        let hashes = WindowHashes::of(&stream.ids, Self::WIDTH);
        let starts = Self::starts_by_hash(&hashes);
        Self {
            stream,
            hashes,
            starts,
        }
    }

    pub(crate) fn duplicates(&self) -> Vec<Duplicate> {
        self.walk()
    }

    fn starts_by_hash(hashes: &[u64]) -> HashMap<u64, Vec<usize>> {
        let mut starts: HashMap<u64, Vec<usize>> = HashMap::new();
        for (at, hash) in hashes.iter().enumerate() {
            starts.entry(*hash).or_default().push(at);
        }
        starts
    }

    fn walk(&self) -> Vec<Duplicate> {
        let mut found = Vec::new();
        let mut at = 0;
        while at < self.hashes.len() {
            let repeat = self.repeat_at(at);
            at += repeat.map_or(1, |repeat| repeat.length);
            found.extend(repeat.map(|repeat| self.duplicate(repeat)));
        }
        found
    }

    fn repeat_at(&self, later: usize) -> Option<Repeat> {
        self.earlier_start(later)
            .map(|earlier| self.stretch(earlier, later))
            .filter(|repeat| self.spans_enough_lines(*repeat) & self.holds_code(*repeat))
    }

    fn earlier_start(&self, later: usize) -> Option<usize> {
        self.starts
            .get(&self.hashes[later])
            .into_iter()
            .flatten()
            .copied()
            .take_while(|earlier| earlier + Self::WIDTH <= later)
            .find(|earlier| self.same_window(*earlier, later))
    }

    fn same_window(&self, earlier: usize, later: usize) -> bool {
        let ids = &self.stream.ids;
        ids[earlier..earlier + Self::WIDTH] == ids[later..later + Self::WIDTH]
    }

    fn stretch(&self, earlier: usize, later: usize) -> Repeat {
        let extra = (Self::WIDTH..self.stream.ids.len())
            .take_while(|step| self.continues(earlier, later, *step))
            .count();
        Repeat {
            earlier,
            later,
            length: Self::WIDTH + extra,
        }
    }

    fn continues(&self, earlier: usize, later: usize, step: usize) -> bool {
        let ids = &self.stream.ids;
        let apart = earlier + step < later;
        apart & (ids.get(earlier + step) == ids.get(later + step))
    }

    fn spans_enough_lines(&self, repeat: Repeat) -> bool {
        let earlier = self.place(repeat.earlier, repeat.length).lines();
        let later = self.place(repeat.later, repeat.length).lines();
        earlier.min(later) >= DuplicateFinder::MIN_LINES
    }

    fn holds_code(&self, repeat: Repeat) -> bool {
        let words = self.stream.words_in(repeat.later, repeat.length);
        words * Self::WORD_SHARE >= repeat.length
    }

    fn duplicate(&self, repeat: Repeat) -> Duplicate {
        Duplicate {
            first: self.place(repeat.earlier, repeat.length),
            second: self.place(repeat.later, repeat.length),
            tokens: repeat.length,
        }
    }

    fn place(&self, start: usize, length: usize) -> Place {
        Place {
            path: self.stream.path(start).to_owned(),
            first_line: self.stream.line(start),
            last_line: self.stream.line(start + length - 1),
        }
    }
}
