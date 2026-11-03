#![allow(clippy::needless_bitwise_bool)]

use super::super::cohesion::{Method, Words};

pub(super) struct TokenIndex {
    line_heads: Vec<usize>,
    line_ends: Vec<usize>,
    parents: Vec<Option<usize>>,
    last_classes: Vec<Option<usize>>,
    in_methods: Vec<bool>,
}

impl TokenIndex {
    const OPENERS: [&'static str; 3] = ["(", "[", "{"];
    const CLOSERS: [&'static str; 3] = [")", "]", "}"];
    const CLASS: &'static str = "class";

    pub(super) fn new(words: &Words<'_>, methods: &[Method]) -> Self {
        Self {
            line_heads: Self::line_heads(words),
            line_ends: Self::line_ends(words),
            parents: Self::parents(words),
            last_classes: Self::last_classes(words),
            in_methods: Self::in_methods(words.len(), methods),
        }
    }

    pub(super) fn line_begin(&self, at: usize) -> usize {
        self.line_heads.get(at).copied().unwrap_or(0)
    }

    pub(super) fn line_end(&self, at: usize) -> usize {
        self.line_ends
            .get(at)
            .copied()
            .unwrap_or(self.line_ends.len())
    }

    pub(super) fn parent(&self, at: usize) -> Option<usize> {
        self.parents.get(at).copied().flatten()
    }

    pub(super) fn last_class(&self, at: usize) -> Option<usize> {
        self.last_classes.get(at).copied().flatten()
    }

    pub(super) fn in_method(&self, at: usize) -> bool {
        self.in_methods.get(at).copied().unwrap_or(false)
    }

    fn line_heads(words: &Words<'_>) -> Vec<usize> {
        let mut heads = Vec::with_capacity(words.len());
        let mut head = 0;
        for at in 0..words.len() {
            head = [head, at][usize::from(words.line_start(at))];
            heads.push(head);
        }
        heads
    }

    fn line_ends(words: &Words<'_>) -> Vec<usize> {
        let mut ends = vec![words.len(); words.len()];
        let mut end = words.len();
        for at in (0..words.len()).rev() {
            end = [end, at + 1][usize::from(words.line_start(at + 1))];
            ends[at] = end;
        }
        ends
    }

    fn parents(words: &Words<'_>) -> Vec<Option<usize>> {
        let mut open = Vec::new();
        (0..words.len())
            .map(|at| Self::parent_of(words, at, &mut open))
            .collect()
    }

    fn parent_of(words: &Words<'_>, at: usize, open: &mut Vec<usize>) -> Option<usize> {
        let text = words.text(at);
        let code = !words.is_literal(at);
        let closes = code & Self::CLOSERS.contains(&text);
        open.pop_if(|_| closes);
        let parent = open.last().copied();
        let opens = code & Self::OPENERS.contains(&text);
        open.extend(opens.then_some(at));
        parent
    }

    fn last_classes(words: &Words<'_>) -> Vec<Option<usize>> {
        let mut last = None;
        (0..words.len())
            .map(|at| {
                let found = (words.text(at) == Self::CLASS) & !words.after_member_access(at);
                last = found.then_some(at).or(last);
                last
            })
            .collect()
    }

    fn in_methods(count: usize, methods: &[Method]) -> Vec<bool> {
        let mut inside = vec![false; count];
        for method in methods {
            let end = method.body.end.min(count);
            inside[method.head.min(end)..end].fill(true);
        }
        inside
    }
}
