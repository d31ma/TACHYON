pub(crate) struct TestAssert;

impl TestAssert {
    pub(super) const JAVASCRIPT: &'static str = include_str!("assert.js");
    pub(super) const PYTHON: &'static str = include_str!("polyglot/python_assert.py");
    pub(super) const JAVA: &'static str = include_str!("polyglot/java_assert.java");
    pub(super) const KOTLIN: &'static str = include_str!("polyglot/kotlin_assert.kt");
    pub(super) const CSHARP: &'static str = include_str!("polyglot/csharp_assert.cs");
    pub(crate) const FSHARP: &'static str = include_str!("polyglot/fsharp_assert.fs");
    pub(super) const RUST: &'static str = concat!(
        include_str!("polyglot/assert.rs"),
        include_str!("polyglot/case_outcome.rs")
    );
}
