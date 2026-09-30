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

fn project_without_pages(elements: &str) -> tempfile::TempDir {
    let root = counter_project(elements);
    fs::remove_dir_all(root.path().join("client/pages")).expect("remove pages");
    root
}

#[test]
fn a_project_that_only_exports_elements_builds_without_a_page() {
    let root = project_without_pages(r#"["shop-counter"]"#);
    let result = WebCompiler::build(root.path(), &BuildOptions::default()).expect("build");
    let output = result.output_directory();

    assert!(output.join(".tachyon/elements/shop-counter.js").is_file());
    assert!(output.join(".tachyon/tac-client.js").is_file());
    assert!(!output.join("index.html").exists());
}

#[test]
fn a_project_without_pages_or_elements_still_asks_for_a_page() {
    let root = project_without_pages("[]");
    let failure = WebCompiler::build(root.path(), &BuildOptions::default()).expect_err("fails");

    assert_eq!(failure.diagnostics()[0].code.to_string(), "TY1002");
}

fn publishing_project() -> tempfile::TempDir {
    let root = counter_project(r#"["shop-counter"]"#);
    write(
        root.path(),
        "client/models/CountChanged.js",
        "/**\n * @typedef {Object} CountChanged\n * @property {number} count\n * @property {string} [note]\n * @property {string[]} tags\n */\nexport {}\n",
    );
    write(
        root.path(),
        "client/services/CounterService.js",
        "@Service()\n@NeedsAll\nexport default class CounterService {\n  /** @type {string} */\n  start\n  /** @type {ApplicationEventPublisher} */\n  applicationEventPublisher\n  count = 0\n\n  add() {\n    /** @type {CountChanged} */\n    const changed = { count: 1, tags: [] }\n    this.applicationEventPublisher.publishEvent(changed)\n  }\n}\n",
    );
    root
}

#[test]
fn an_exported_element_has_typescript_declarations_for_its_tag_props_and_events() {
    let root = publishing_project();
    let result = WebCompiler::build(root.path(), &BuildOptions::default()).expect("build");
    let path = result
        .output_directory()
        .join(".tachyon/elements/shop-counter.d.ts");
    let declaration = fs::read_to_string(path).expect("declaration");

    assert!(
        declaration.contains(
            "export interface CountChanged {\n  count: number\n  note?: string\n  tags: string[]\n}\n"
        ),
        "{declaration}"
    );
    assert!(
        declaration.contains("  \"count-changed\": CustomEvent<CountChanged>\n"),
        "{declaration}"
    );
    assert!(
        declaration.contains("export declare class ShopCounterElement extends HTMLElement {\n  label: unknown\n  start: unknown\n"),
        "{declaration}"
    );
    assert!(
        declaration.contains(
            "interface HTMLElementTagNameMap {\n    \"shop-counter\": ShopCounterElement\n"
        ),
        "{declaration}"
    );
}

#[test]
fn a_named_slot_becomes_a_native_named_slot_in_the_shadow_root() {
    let root = counter_project(r#"["shop-counter"]"#);
    write(
        root.path(),
        "client/components/shop/counter/tac.html",
        r#"<section aria-label="Counter"><h2><slot name="title"></slot></h2><p>{counter.count}</p></section>"#,
    );
    fs::remove_dir_all(root.path().join("client/pages")).expect("remove pages");
    let result = WebCompiler::build(root.path(), &BuildOptions::default()).expect("build");
    let path = result
        .output_directory()
        .join(".tachyon/elements/shop-counter.js");
    let module = fs::read_to_string(path).expect("element");

    assert!(module.contains(r#""slot":[]"#), "{module}");
    assert!(
        module.contains(r#""slots":{"title":[{"attributes":[{"name":"name","value":"title"}],"children":[],"k":"element","tag":"slot","void":false}]}"#),
        "{module}"
    );
    assert!(
        module.contains(r#"{"k":"slot","name":"title"}"#),
        "{module}"
    );
}
