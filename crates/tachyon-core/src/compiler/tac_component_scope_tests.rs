use super::{BuildOptions, WebCompiler};
use std::path::Path;

struct TacComponentScopeTests;

impl TacComponentScopeTests {
    fn write(root: &Path, relative: &str, source: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap_or_else(|| panic!("parent")))
            .unwrap_or_else(|failure| panic!("component directory: {failure}"));
        std::fs::write(path, source)
            .unwrap_or_else(|failure| panic!("component source: {failure}"));
    }

    fn project(page: &str, parent: &str) -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap_or_else(|failure| panic!("project: {failure}"));
        Self::write(root.path(), "client/pages/tac.html", page);
        Self::write(
            root.path(),
            "client/components/parent/panel/tac.html",
            parent,
        );
        Self::write(
            root.path(),
            "client/components/plain/child/tac.html",
            "<p class=\"probe\">Unstyled child</p>",
        );
        root
    }

    fn build(root: &Path) -> std::path::PathBuf {
        WebCompiler::build(root, &BuildOptions::default())
            .unwrap_or_else(|failure| panic!("ordinary component bundle: {failure}"))
            .output_directory()
            .to_path_buf()
    }

    fn document(output: &Path) -> String {
        std::fs::read_to_string(output.join("index.html"))
            .unwrap_or_else(|failure| panic!("compiled document: {failure}"))
    }
}

#[test]
fn tac_component_scope_unstyled_nested_child_has_an_ownership_boundary() {
    let root =
        TacComponentScopeTests::project("<parent-panel />", "<section><plain-child /></section>");
    let output = TacComponentScopeTests::build(root.path());
    let document = TacComponentScopeTests::document(&output);
    assert_eq!(document.matches("\"scope\":true").count(), 2, "{document}");
    assert!(!document.contains("\"scope\":false"), "{document}");
    assert!(!output.join(".tachyon/components.css").exists());
}

#[test]
fn tac_component_scope_parent_selectors_stop_at_nested_component_boundaries() {
    let root = TacComponentScopeTests::project(
        "<parent-panel />",
        "<section><p class=\"probe\">Owned</p><plain-child /></section>",
    );
    TacComponentScopeTests::write(
        root.path(),
        "client/components/parent/panel/tac.css",
        ".probe { background: rgb(240, 20, 20); }",
    );
    let output = TacComponentScopeTests::build(root.path());
    let stylesheet = std::fs::read_to_string(output.join(".tachyon/components.css"))
        .unwrap_or_else(|failure| panic!("component stylesheet: {failure}"));
    assert!(
        stylesheet
            .contains("@scope ([data-tac-scope=\"parent-panel\"]) to (:scope [data-tac-scope])"),
        "{stylesheet}"
    );
}

#[test]
fn tac_component_scope_slotted_child_keeps_its_own_boundary() {
    let root = TacComponentScopeTests::project(
        "<parent-panel><plain-child /></parent-panel>",
        "<section><slot></slot></section>",
    );
    let output = TacComponentScopeTests::build(root.path());
    let document = TacComponentScopeTests::document(&output);
    assert_eq!(document.matches("\"scope\":true").count(), 2, "{document}");
    assert!(!document.contains("\"scope\":false"), "{document}");
}

#[test]
fn tac_component_scope_component_with_only_a_component_root_keeps_both_owners() {
    let root = TacComponentScopeTests::project("<parent-panel />", "<plain-child />");
    let output = TacComponentScopeTests::build(root.path());
    let document = TacComponentScopeTests::document(&output);
    assert_eq!(document.matches("\"scope\":true").count(), 2, "{document}");
    assert!(!document.contains("\"scope\":false"), "{document}");
}

#[test]
fn tac_component_scope_neighbor_instances_keep_independent_ownership_plans() {
    let root = TacComponentScopeTests::project(
        "<parent-panel /><parent-panel /><plain-child />",
        "<section><plain-child /></section>",
    );
    let output = TacComponentScopeTests::build(root.path());
    let document = TacComponentScopeTests::document(&output);
    assert_eq!(document.matches("\"scope\":true").count(), 5, "{document}");
    assert!(!document.contains("\"scope\":false"), "{document}");
}
