use super::MigrationFix;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn project(files: &[(&str, &str)]) -> TempDir {
    let project = tempfile::tempdir().expect("project");
    for (relative, contents) in files {
        let path = project.path().join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        fs::write(path, contents).expect("source");
    }
    project
}

fn fix(root: &Path) -> String {
    let planned = MigrationFix::plan(root).expect("plan");
    let text = planned.to_text(false);
    planned.write(root).expect("write");
    text
}

fn read(root: &Path, relative: &str) -> String {
    fs::read_to_string(root.join(relative)).expect(relative)
}

fn passes(root: &Path, relative: &str) {
    let source = read(root, relative);
    let path = Path::new(relative);
    let checked = crate::annotation_layout::InlineModelCheck::refuse_inline_models(path, &source)
        .and_then(|()| crate::model_layout::validate(path, &source));
    assert!(checked.is_ok(), "{relative}: {checked:?}\n{source}");
}

const CART_TOTAL: &str = "/**\n * @typedef {Object} CartTotal\n * @property {number} total\n */\n";

fn service(name: &str, typedefs: &str) -> String {
    format!(
        "{typedefs}@Service\nexport default class {name} {{\n  /** @param {{CartTotal}} value */\n  @OnEvent\n  changed(value) {{}}\n}}\n"
    )
}

#[test]
fn identical_inline_typedefs_become_one_javascript_model_file() {
    let project = project(&[
        (
            "client/services/CartService.js",
            &service("CartService", CART_TOTAL),
        ),
        (
            "client/services/BadgeService.js",
            &service("BadgeService", CART_TOTAL),
        ),
    ]);
    let root = project.path();
    let text = fix(root);
    assert!(text.contains("client/models/CartTotal.js"), "{text}");
    assert_eq!(
        read(root, "client/models/CartTotal.js"),
        format!("{CART_TOTAL}export {{}}\n")
    );
    assert_eq!(
        read(root, "client/services/CartService.js"),
        service("CartService", "")
    );
    assert_eq!(
        read(root, "client/services/BadgeService.js"),
        service("BadgeService", "")
    );
    for relative in [
        "client/models/CartTotal.js",
        "client/services/CartService.js",
        "client/services/BadgeService.js",
    ] {
        passes(root, relative);
    }
}

#[test]
fn a_typedef_that_copies_a_model_of_another_language_imports_that_model() {
    let project = project(&[
        (
            "client/models/CartTotal.ts",
            "export interface CartTotal {\n  total: number\n}\n",
        ),
        (
            "client/services/CartService.js",
            &service("CartService", CART_TOTAL),
        ),
    ]);
    let root = project.path();
    fix(root);
    assert!(!root.join("client/models/CartTotal.js").exists());
    assert_eq!(
        read(root, "client/services/CartService.js"),
        service(
            "CartService",
            "/** @import { CartTotal } from '/models/CartTotal.ts' */\n"
        )
    );
    passes(root, "client/services/CartService.js");
}

#[test]
fn a_typedef_that_copies_a_javascript_model_is_removed() {
    let project = project(&[
        (
            "server/models/CartTotal.js",
            &format!("{CART_TOTAL}export {{}}\n"),
        ),
        (
            "server/services/CartService.js",
            &service("CartService", CART_TOTAL),
        ),
    ]);
    let root = project.path();
    fix(root);
    assert_eq!(
        read(root, "server/services/CartService.js"),
        service("CartService", "")
    );
}

const ORDER_SUMMARY: &str =
    "/**\n * @typedef {Object} OrderSummary\n * @property {CartTotal} total\n */\n";

#[test]
fn a_new_javascript_model_that_uses_a_typescript_model_imports_it() {
    let project = project(&[
        (
            "client/models/CartTotal.ts",
            "export interface CartTotal {\n  total: number\n}\n",
        ),
        (
            "client/services/OrderService.js",
            &service("OrderService", ORDER_SUMMARY),
        ),
    ]);
    let root = project.path();
    fix(root);
    assert_eq!(
        read(root, "client/models/OrderSummary.js"),
        format!(
            "/** @import {{ CartTotal }} from '/models/CartTotal.ts' */\n{ORDER_SUMMARY}export {{}}\n"
        )
    );
    assert_eq!(
        read(root, "client/services/OrderService.js"),
        service("OrderService", "")
    );
    for relative in [
        "client/models/OrderSummary.js",
        "client/services/OrderService.js",
    ] {
        passes(root, relative);
    }
}

#[test]
fn conflicting_typedefs_are_reported_and_left_unchanged() {
    let other = "/**\n * @typedef {Object} CartTotal\n * @property {string} total\n */\n";
    let project = project(&[
        (
            "client/models/CartTotal.ts",
            "export interface CartTotal {\n  total: number\n}\n",
        ),
        (
            "client/services/CartService.js",
            &service("CartService", other),
        ),
        (
            "server/services/BadgeService.js",
            &service("BadgeService", CART_TOTAL),
        ),
        (
            "server/services/CartService.js",
            &service("CartService", other),
        ),
    ]);
    let root = project.path();
    let text = fix(root);
    assert_eq!(
        read(root, "client/services/CartService.js"),
        service("CartService", other)
    );
    assert!(
        text.contains("'CartTotal' differs from the model in 'client/models/CartTotal.ts'"),
        "{text}"
    );
    assert!(
        text.contains("'CartTotal' differs from the one in 'server/services/BadgeService.js'"),
        "{text}"
    );
    assert_eq!(
        read(root, "server/models/CartTotal.js"),
        format!("{CART_TOTAL}export {{}}\n")
    );
    assert_eq!(
        read(root, "server/services/CartService.js"),
        service("CartService", other)
    );
}
