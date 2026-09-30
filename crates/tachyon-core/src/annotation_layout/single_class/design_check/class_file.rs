#![allow(clippy::needless_bitwise_bool)]

use super::super::super::Input;
use super::super::Tokens;
use super::super::cohesion::{CohesionCheck, Method, Spelling, Words};
use super::super::duplication::ProjectSource;
use super::design_problem::DesignProblem;
use super::design_rule::DesignRule;
use super::path_parts::PathParts;
use super::token_index::TokenIndex;
use std::path::Path;

pub(super) struct ClassFile<'a> {
    pub(super) path: &'a str,
    pub(super) text: &'a str,
    pub(super) realm: &'a str,
    pub(super) folder: &'a str,
    pub(super) name: &'a str,
    pub(super) extension: &'a str,
    pub(super) authored_extension: &'a str,
    pub(super) tokens: Tokens<'a>,
    pub(super) words: Words<'a>,
    pub(super) spelling: Spelling,
    pub(super) methods: Vec<Method>,
    pub(super) index: TokenIndex,
}

impl<'a> ClassFile<'a> {
    const HELPER_FOLDER: &'static str = "helpers";
    const HELPER_MARKER: &'static str = "Helper";

    pub(super) fn read(source: &'a ProjectSource) -> Option<Self> {
        let parts = PathParts::of(&source.path)?;
        let spelling = Spelling::of(parts.extension)?;
        let input = Input::new(Path::new(&source.path), &source.text);
        let tokens = Tokens::read(&input).ok()?;
        let production = !tokens.has_test_marker() & Self::is_role_class(&parts, &tokens);
        let words = Words::read(&tokens, &source.text);
        let methods = CohesionCheck::methods(&words, &spelling);
        let index = TokenIndex::new(&words, &methods);
        production.then(|| Self {
            path: &source.path,
            text: &source.text,
            realm: parts.realm,
            folder: parts.folder,
            name: parts.name,
            extension: parts.extension,
            authored_extension: &source.authored_extension,
            tokens,
            words,
            spelling,
            methods,
            index,
        })
    }

    fn is_role_class(parts: &PathParts, tokens: &Tokens) -> bool {
        let helper_folder = parts.folder == Self::HELPER_FOLDER;
        !helper_folder | tokens.has_role_marker(Self::HELPER_MARKER)
    }

    pub(super) fn problem(
        &self,
        rule: DesignRule,
        at: usize,
        message: String,
        help: String,
    ) -> DesignProblem {
        let start = self.words.start(at);
        let end = start + self.words.text(at).len();
        DesignProblem {
            rule,
            path: self.path.to_owned(),
            start,
            end,
            line: self.line(start),
            message,
            help,
        }
    }

    pub(super) fn line(&self, start: usize) -> usize {
        let before = self.text.get(..start).unwrap_or("");
        before.matches('\n').count() + 1
    }

    pub(super) fn first_word(&self, wanted: &str) -> usize {
        self.find_word(wanted).unwrap_or(0)
    }

    pub(super) fn find_word(&self, wanted: &str) -> Option<usize> {
        (0..self.words.len()).find(|&at| self.words.text(at) == wanted)
    }
}
