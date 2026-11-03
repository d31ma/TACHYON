#![allow(clippy::needless_bitwise_bool)]

use super::pieces::Pieces;

pub(super) struct Shape {
    pub(super) start: usize,
    pub(super) end: usize,
    bindings: Vec<(String, String)>,
}

impl Shape {
    const NAME: u8 = b'@';
    const CHAIN: u8 = b'%';
    const VALUE: u8 = b'#';

    pub(super) fn read(pieces: &Pieces<'_>, start: usize, template: &str) -> Option<Self> {
        let shape = Self::walk(pieces, start, template)?;
        Some(shape).filter(Self::is_consistent)
    }

    pub(super) fn get(&self, name: &str) -> &str {
        self.bindings
            .iter()
            .find(|(bound, _)| bound == name)
            .map_or("", |(_, text)| text.as_str())
    }

    pub(super) fn with(mut self, name: &str, value: &str) -> Self {
        self.bindings.push((name.to_owned(), value.to_owned()));
        self
    }

    pub(super) fn fill(&self, template: &str) -> String {
        self.bindings
            .iter()
            .fold(template.to_owned(), |text, (name, value)| {
                text.replace(&format!("{{{name}}}"), value)
            })
    }

    fn walk(pieces: &Pieces<'_>, start: usize, template: &str) -> Option<Self> {
        let mut shape = Self {
            start,
            end: start,
            bindings: Vec::new(),
        };
        for token in template.split_whitespace() {
            let (next, bound) = Self::step(pieces, shape.end, token)?;
            shape.end = next;
            shape.bindings.extend(bound);
        }
        Some(shape)
    }

    fn step(
        pieces: &Pieces<'_>,
        at: usize,
        token: &str,
    ) -> Option<(usize, Option<(String, String)>)> {
        let name = token.get(1..).unwrap_or("").to_owned();
        match token.as_bytes().first().copied() {
            Some(Self::NAME) => Some(at)
                .filter(|&index| pieces.is_name(index))
                .map(|index| (index + 1, Some((name, pieces.text(index).to_owned())))),
            Some(Self::CHAIN) => Self::chain(pieces, at)
                .map(|end| (end, Some((name, pieces.slice(at, end).to_owned())))),
            Some(Self::VALUE) => Some(at)
                .filter(|&index| pieces.is_literal(index) | pieces.is_number(index))
                .map(|index| (index + 1, Some((name, pieces.text(index).to_owned())))),
            _ => pieces.is(at, token).then_some((at + 1, None)),
        }
    }

    fn chain(pieces: &Pieces<'_>, at: usize) -> Option<usize> {
        let first = Some(at).filter(|&index| pieces.is_name(index))?;
        let mut end = first + 1;
        while pieces.is(end, ".")
            & pieces.is_name(end + 1)
            & pieces.touches(end)
            & pieces.touches(end + 1)
        {
            end += 2;
        }
        Some(end)
    }

    fn is_consistent(&self) -> bool {
        self.bindings.iter().all(|(name, text)| {
            self.bindings
                .iter()
                .filter(|(other, _)| other == name)
                .all(|(_, value)| value == text)
        })
    }
}
