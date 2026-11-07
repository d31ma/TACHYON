use crate::compiler::{BuildOptions, WebCompiler};
use std::fs;
use std::path::Path;

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("folder");
    fs::write(path, contents).expect("file");
}

fn shell_project(page: &str) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("project");
    write(
        root.path(),
        "client/components/site/shell/tac.html",
        r#"<main><header><slot name="title"></slot></header><slot></slot></main>"#,
    );
    write(
        root.path(),
        "client/components/page/title/tac.html",
        "<h1>Shop</h1>",
    );
    write(
        root.path(),
        "client/components/product/card/tac.html",
        "<article>Anvil</article>",
    );
    write(root.path(), "client/pages/tac.html", page);
    root
}

fn build_message(root: &Path) -> String {
    let failure = WebCompiler::build(root, &BuildOptions::default()).expect_err("fails");
    let problem = &failure.diagnostics()[0];
    format!("{} {}", problem.code, problem.message)
}

#[test]
fn a_child_with_a_slot_attribute_goes_into_the_slot_with_that_name() {
    let root = shell_project(
        "<site-shell>\n  <page-title slot=\"title\" />\n  <product-card />\n</site-shell>\n",
    );
    let result = WebCompiler::build(root.path(), &BuildOptions::default()).expect("build");
    let page = fs::read_to_string(result.output_directory().join("index.html")).expect("page");

    assert!(page.contains(r#"{"k":"slot","name":"title"}"#), "{page}");
    assert!(
        page.contains(r#""slots":{"title":[{"k":"component""#),
        "{page}"
    );
}

#[test]
fn a_child_for_a_slot_the_component_does_not_declare_fails_the_build() {
    let root = shell_project("<site-shell><page-title slot=\"footer\" /></site-shell>\n");

    assert_eq!(
        build_message(root.path()),
        "TY1404 Component '<site-shell>' received children for slot 'footer' but declares no <slot name=\"footer\">."
    );
}

#[test]
fn a_slot_name_must_be_fixed_and_not_empty() {
    for template in [
        r#"<main><slot name=""></slot></main>"#,
        r#"<main><slot id="a"></slot></main>"#,
    ] {
        let root = shell_project("<site-shell><product-card /></site-shell>\n");
        write(
            root.path(),
            "client/components/site/shell/tac.html",
            template,
        );

        assert!(
            build_message(root.path()).starts_with("TY1302 "),
            "{template}"
        );
    }
}
