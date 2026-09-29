use crate::compiler::{BuildOptions, WebCompiler};
use std::fs;
use std::path::Path;

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("folder");
    fs::write(path, contents).expect("file");
}

fn counter_project(elements: &str) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("project");
    let manifest = format!(r#"{{"name":"Shop","tac":{{"elements":{elements}}}}}"#);
    write(root.path(), "client/manifest.json", &manifest);
    write(
        root.path(),
        "client/pages/tac.html",
        "<shop-counter label=\"Page\" start=\"1\" />",
    );
    write(
        root.path(),
        "client/components/shop/counter/tac.html",
        r#"<section aria-label="Counter"><h2>{label}</h2><p>{counter.count}</p><slot></slot></section>"#,
    );
    write(
        root.path(),
        "client/components/counter/tac.html",
        "<p>plain</p>",
    );
    write(
        root.path(),
        "client/services/CounterService.js",
        "@Service()\n@NeedsAll\nexport default class CounterService {\n  /** @type {string} */\n  start\n  count = 0\n}\n",
    );
    root
}

fn build_message(elements: &str) -> String {
    let root = counter_project(elements);
    let failure = WebCompiler::build(root.path(), &BuildOptions::default()).expect_err("fails");
    let problem = &failure.diagnostics()[0];
    format!("{} {}", problem.code, problem.message)
}

#[test]
fn an_exported_component_becomes_one_module_beside_the_shared_runtime() {
    let root = counter_project(r#"["shop-counter"]"#);
    let result = WebCompiler::build(root.path(), &BuildOptions::default()).expect("build");
    let output = result.output_directory().join(".tachyon");
    let module = fs::read_to_string(output.join("elements/shop-counter.js")).expect("element");

    assert!(
        module.starts_with("import { TacElements } from '../tac-client.js'\n"),
        "{module}"
    );
    assert!(module.contains(r#""props":["label","start"]"#), "{module}");
    assert!(module.contains(r#""name":"shop-counter","#), "{module}");
    assert!(module.contains(r#""exported":true"#), "{module}");
    assert!(module.contains(r#""mount":"load""#), "{module}");
    assert!(
        module.contains(
            r#""slot":[{"attributes":[],"children":[],"k":"element","tag":"slot","void":false}]"#
        ),
        "{module}"
    );
    assert!(output.join("tac-client.js").is_file());
}

#[test]
fn an_element_name_without_a_hyphen_fails_the_build() {
    let message = build_message(r#"["counter"]"#);

    assert!(
        message.starts_with("TY1358 `counter` cannot be a custom element"),
        "{message}"
    );
}

#[test]
fn an_element_that_names_no_component_fails_the_build() {
    let message = build_message(r#"["shop-basket"]"#);

    assert_eq!(
        message,
        "TY1358 tac.elements names `shop-basket`, but no component has that tag."
    );
}
