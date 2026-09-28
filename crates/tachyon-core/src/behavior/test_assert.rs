pub(super) struct TestAssert;

impl TestAssert {
    pub(super) const JAVASCRIPT: &'static str = include_str!("assert.js");
    pub(super) const PYTHON: &'static str = include_str!("polyglot/python_assert.py");
    pub(super) const RUST: &'static str = include_str!("polyglot/rust_assert.rs");
}
