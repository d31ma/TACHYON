#![allow(clippy::needless_bitwise_bool)]

use super::super::super::super::Input;
use super::super::super::Tokens;
use super::super::super::cohesion::{CohesionCheck, Method, Spelling, Words};
use super::super::super::duplication::ProjectSource;
use super::super::design_problem::DesignProblem;
use super::super::design_rule::DesignRule;
use super::test_language::TestLanguage;
use std::path::Path;

pub(super) struct TestFile<'a> {
    pub(super) path: &'a str,
    pub(super) text: &'a str,
    pub(super) class_name: &'a str,
    pub(super) words: Words<'a>,
    pub(super) spelling: Spelling,
    pub(super) language: TestLanguage,
    pub(super) methods: Vec<Method>,
}

impl<'a> TestFile<'a> {
    const TEST_FOLDERS: [&'static str; 2] = ["client/tests/", "server/tests/"];
    const TEST_SUFFIX: &'static str = "Test";

    pub(super) fn read(source: &'a ProjectSource) -> Option<Self> {
        let path = Path::new(&source.path);
        let class_name = path.file_stem()?.to_str()?;
        let extension = source.extension();
        let language = TestLanguage::of(extension)?;
        let spelling = Spelling::of(extension)?;
        let input = Input::new(path, &source.text);
        let tokens = Tokens::read(&input).ok()?;
        let unit = Self::is_unit_path(&source.path, class_name) & tokens.has_test_marker();
        let words = Words::read(&tokens, &source.text);
        let methods = CohesionCheck::methods(&words, &spelling);
        unit.then(|| Self {
            path: &source.path,
            text: &source.text,
            class_name,
            words,
            spelling,
            language,
            methods,
        })
    }

    fn is_unit_path(path: &str, class_name: &str) -> bool {
        let in_folder = Self::TEST_FOLDERS
            .iter()
            .any(|folder| path.starts_with(folder));
        in_folder & class_name.ends_with(Self::TEST_SUFFIX)
    }

    pub(super) fn problem(&self, at: usize, message: String, help: String) -> DesignProblem {
        let start = self.words.start(at);
        let end = start + self.words.text(at).len();
        let before = self.text.get(..start).unwrap_or("");
        DesignProblem {
            rule: DesignRule::Tests,
            path: self.path.to_owned(),
            start,
            end,
            line: before.matches('\n').count() + 1,
            message,
            help,
        }
    }
}
