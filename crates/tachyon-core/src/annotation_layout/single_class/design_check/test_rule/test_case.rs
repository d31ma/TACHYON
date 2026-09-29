#![allow(clippy::needless_bitwise_bool)]

use super::super::super::cohesion::Method;
use super::super::design_problem::DesignProblem;
use super::test_file::TestFile;
use std::ops::Range;

pub(super) struct TestCase<'f, 'a> {
    file: &'f TestFile<'a>,
    method: &'f Method,
}

impl<'f, 'a> TestCase<'f, 'a> {
    const ASSERT_CLASS: &'static str = "Assert";
    const MEMBER_ACCESS: [&'static str; 2] = [".", ":"];
    const MACRO_MARK: &'static str = "!";
    const TERNARY_MARK: &'static str = "?";
    const TERNARY_NAME: &'static str = "? :";
    const LOGIC_HELP: &'static str = "A test case prepares, calls and checks, with no branch and no loop. Write one case for each path, or list the inputs in ValueSource so that Tachyon runs one case for each input (ADR 0080).";

    pub(super) fn cases(file: &'f TestFile<'a>) -> Vec<Self> {
        file.methods
            .iter()
            .filter(|method| method.name.starts_with(file.language.case_prefix))
            .map(|method| Self { file, method })
            .collect()
    }

    pub(super) fn problems(&self) -> Vec<DesignProblem> {
        let logic = self.first_logic().map(|at| self.logic_problem(at));
        let unchecked = (!self.asserts()).then(|| self.missing_assert_problem());
        logic.into_iter().chain(unchecked).collect()
    }

    fn body(&self) -> Range<usize> {
        self.method.body.clone()
    }

    fn first_logic(&self) -> Option<usize> {
        self.body().find(|&at| self.is_logic(at))
    }

    fn is_logic(&self, at: usize) -> bool {
        self.is_logic_word(at) | self.is_ternary(at)
    }

    fn is_logic_word(&self, at: usize) -> bool {
        let words = &self.file.words;
        let listed = self.file.language.logic_words.contains(&words.text(at));
        listed & words.is_name(at) & !words.after_member_access(at)
    }

    fn is_ternary(&self, at: usize) -> bool {
        let words = &self.file.words;
        let mark = words.text(at) == Self::TERNARY_MARK;
        self.file.spelling.has_ternary() & mark & words.spaced(at)
    }

    fn asserts(&self) -> bool {
        self.body().any(|at| self.is_assertion(at))
    }

    fn is_assertion(&self, at: usize) -> bool {
        self.calls_assert_class(at) | self.is_statement_assert(at) | self.is_macro_assert(at)
    }

    fn calls_assert_class(&self, at: usize) -> bool {
        let words = &self.file.words;
        let named = words.text(at) == Self::ASSERT_CLASS;
        named & Self::MEMBER_ACCESS.contains(&words.text(at + 1)) & !words.after_member_access(at)
    }

    fn is_statement_assert(&self, at: usize) -> bool {
        let words = &self.file.words;
        let listed = self
            .file
            .language
            .statement_asserts
            .contains(&words.text(at));
        listed & words.is_name(at) & !words.after_member_access(at)
    }

    fn is_macro_assert(&self, at: usize) -> bool {
        let words = &self.file.words;
        let listed = self.file.language.macro_asserts.contains(&words.text(at));
        listed & (words.text(at + 1) == Self::MACRO_MARK)
    }

    fn name_word(&self) -> usize {
        let words = &self.file.words;
        (self.method.head..self.method.body.start)
            .find(|&at| words.text(at) == self.method.name)
            .unwrap_or(self.method.head)
    }

    fn logic_problem(&self, at: usize) -> DesignProblem {
        let word = self.logic_name(at);
        let message = format!(
            "The test case `{}` in {} holds logic: `{word}`.",
            self.method.name, self.file.class_name
        );
        self.file.problem(at, message, Self::LOGIC_HELP.to_owned())
    }

    fn logic_name(&self, at: usize) -> &str {
        if self.is_ternary(at) {
            return Self::TERNARY_NAME;
        }
        self.file.words.text(at)
    }

    fn missing_assert_problem(&self) -> DesignProblem {
        let message = format!(
            "The test case `{}` in {} has no assertion.",
            self.method.name, self.file.class_name
        );
        let help = self.file.language.assert_help.to_owned();
        self.file.problem(self.name_word(), message, help)
    }
}
