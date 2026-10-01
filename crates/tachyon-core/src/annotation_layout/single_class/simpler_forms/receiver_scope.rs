#![allow(clippy::needless_bitwise_bool)]

use super::super::cohesion::{Method, Words};

pub(super) struct ReceiverScope<'r, 's> {
    words: &'r Words<'s>,
    methods: &'r [Method],
    method: &'r Method,
}

impl<'r, 's> ReceiverScope<'r, 's> {
    const MEMBER: &'static str = ".";
    const BLOCK: &'static str = "{";

    pub(super) fn new(words: &'r Words<'s>, methods: &'r [Method], method: &'r Method) -> Self {
        Self {
            words,
            methods,
            method,
        }
    }

    pub(super) fn permits(&self, at: usize) -> bool {
        let member = self.words.text(at + 2);
        let owned = self.method.owns(at);
        let named = self.words.is_name(at + 2);
        let plain_receiver = !self.words.after_member_access(at);
        let declared = self.declared_member(member);
        let shadowed = self.possibly_shadowed(member);
        let nested = self.nested_block(at);
        let extension = self.extension_method();
        owned & named & plain_receiver & declared & !shadowed & !nested & !extension
    }

    fn declared_member(&self, name: &str) -> bool {
        let depth = self.words.depth(self.method.head);
        let owner = self.owner(depth);
        self.words
            .inside_of(owner)
            .any(|at| self.member_at(at, depth, name))
    }

    fn owner(&self, depth: usize) -> Option<usize> {
        (0..self.method.head).rev().find(|&at| {
            (self.words.text(at) == Self::BLOCK)
                & (self.words.depth(at) + 1 == depth)
                & self.words.inside(at).contains(&self.method.head)
        })
    }

    fn member_at(&self, at: usize, depth: usize, name: &str) -> bool {
        let named = (self.words.text(at) == name) & !self.words.is_literal(at);
        let direct = self.words.depth(at) == depth;
        let body = self.methods.iter().any(|method| method.body.contains(&at));
        named & direct & !body
    }

    fn possibly_shadowed(&self, name: &str) -> bool {
        (self.method.head + 1..self.method.body.end).any(|at| {
            (self.words.text(at) == name)
                & !self.words.is_literal(at)
                & !self.words.after_member_access(at)
        })
    }

    fn nested_block(&self, at: usize) -> bool {
        (self.method.body.start..at).any(|open| {
            (self.words.text(open) == Self::BLOCK) & self.words.inside(open).contains(&at)
        })
    }

    fn extension_method(&self) -> bool {
        self.words
            .has_word(self.method.head..self.method.body.start, &[Self::MEMBER])
    }

    pub(super) fn is_access(words: &Words<'_>, at: usize, receiver: &str) -> bool {
        let named = (words.text(at) == receiver) & !words.is_literal(at);
        named & (words.text(at + 1) == Self::MEMBER)
    }
}
