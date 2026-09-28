pub(super) struct TestAssert;

impl TestAssert {
    pub(super) const JAVASCRIPT: &'static str = include_str!("assert.js");
    pub(super) const PYTHON: &'static str = include_str!("polyglot/python_assert.py");
    pub(super) const RUST: &'static str = concat!(
        include_str!("polyglot/assert.rs"),
        include_str!("polyglot/case_outcome.rs")
    );
}
