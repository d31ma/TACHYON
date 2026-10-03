#![allow(clippy::expect_used)]

use super::{ComponentRegistry, TemplateFrontend};
use crate::{BuildOptions, Failure, NativeBuildOptions, NativeCompiler, WebCompiler};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

fn write(root: &Path, name: &str, source: &str) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().expect("source parent")).expect("source directory");
    fs::write(path, source).expect("source file");
}

fn project(markup: &str) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("component style ownership project");
    write(root.path(), "client/pages/tac.html", "<card-view />");
    write(root.path(), "client/components/card/view/tac.html", markup);
    root
}

fn resource_project(markup: &str) -> tempfile::TempDir {
    let root = project(markup);
    write(
        root.path(),
        "client/services/ResourceService.js",
        "@Service\nexport default class ResourceService { url = '/shared/favicon.svg'; kind = 'image'; }",
    );
    root
}

fn failure(root: &Path) -> Failure {
    let failure = WebCompiler::build(root, &BuildOptions::default())
        .expect_err("component stylesheet ownership");
    assert!(
        failure
            .diagnostics()
            .iter()
            .any(|problem| problem.code.number() == 1403),
        "{failure}"
    );
    failure
}

fn refused(markup: &str) -> String {
    let root = project(markup);
    let failure = failure(root.path());
    let text = failure.to_string();
    assert!(
        text.contains("client/components/card/view/tac.html"),
        "{text}"
    );
    assert!(text.contains("tac.css"), "{text}");
    assert!(text.contains("tac.styleSources"), "{text}");
    text
}

fn accepted(markup: &str) {
    let root = project(markup);
    let result = WebCompiler::build(root.path(), &BuildOptions::default());
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn component_style_ownership_refuses_real_unscoped_style_element() {
    assert!(
        refused("<section><style>.probe{display:grid}</style><p>Card</p></section>")
            .contains("style element")
    );
}

#[test]
fn component_style_ownership_refuses_unlisted_global_stylesheet() {
    let root = project("<link rel=\"stylesheet\" href=\"/shared/hidden.css\"><p>Card</p>");
    write(
        root.path(),
        "client/shared/hidden.css",
        ".probe{display:grid}",
    );
    assert!(failure(root.path()).to_string().contains("stylesheet"));
}

#[test]
fn component_style_ownership_diagnostic_retains_real_element_range() {
    let markup = "<p>Before</p><style>.probe{display:grid}</style>";
    let root = project(markup);
    let failure = failure(root.path());
    let problem = failure
        .diagnostics()
        .iter()
        .find(|problem| problem.code.number() == 1403)
        .expect("ownership diagnostic");
    assert_eq!(
        problem.spans[0].file,
        "client/components/card/view/tac.html"
    );
    assert_eq!(problem.spans[0].start, 13);
    assert_eq!(problem.spans[0].end, markup.len() as u64);
}

#[test]
fn component_style_ownership_stylesheet_relation_is_a_case_insensitive_token() {
    assert!(
        refused("<link rel=\"alternate StyleSheet\" href=\"/shared/hidden.css\">")
            .contains("stylesheet")
    );
}

#[test]
fn component_style_ownership_entity_encoded_stylesheet_relation_is_checked() {
    assert!(
        refused("<link rel=\"style&#115;heet\" href=\"/shared/hidden.css\">")
            .contains("stylesheet")
    );
}

#[test]
fn component_style_ownership_preload_as_style_is_refused() {
    assert!(
        refused("<link rel=\"preload\" as=\"STYLE\" href=\"/shared/hidden.css\">")
            .contains("stylesheet")
    );
}

#[test]
fn component_style_ownership_dynamic_relation_cannot_hide_stylesheet() {
    assert!(
        refused("<link :rel=\"resource.rel\" as=\"image\" :href=\"resource.url\">")
            .contains("dynamic resource role")
    );
}

#[test]
fn component_style_ownership_dynamic_preload_kind_cannot_hide_style() {
    assert!(
        refused("<link rel=\"preload\" :as=\"resource.kind\" :href=\"resource.url\">")
            .contains("dynamic resource role")
    );
}

#[test]
fn component_style_ownership_literal_dynamic_stylesheet_is_refused() {
    assert!(
        refused("<link :rel=\"'stylesheet'\" href=\"/shared/hidden.css\">").contains("stylesheet")
    );
}

#[test]
fn component_style_ownership_false_control_does_not_exempt_style_source() {
    assert!(
        refused("<if :when=\"false\"><style>.probe{display:grid}</style></if><p>Card</p>")
            .contains("style element")
    );
}

#[test]
fn component_style_ownership_loop_content_is_checked_recursively() {
    assert!(
        refused("<for :each=\"item in items\"><link rel=\"stylesheet\" :href=\"item.url\"></for>")
            .contains("stylesheet")
    );
}

#[test]
fn component_style_ownership_projected_slot_content_is_checked() {
    let root =
        project("<card-shell><style slot=\"title\">.probe{display:grid}</style></card-shell>");
    write(
        root.path(),
        "client/components/card/shell/tac.html",
        "<section><slot name=\"title\" /></section>",
    );
    assert!(failure(root.path()).to_string().contains("style element"));
}

#[test]
fn component_style_ownership_unreachable_components_are_checked() {
    let root = project("<p>Safe card</p>");
    write(
        root.path(),
        "client/components/unreachable/card/tac.html",
        "<style>.probe{display:grid}</style>",
    );
    let error = failure(root.path()).to_string();
    assert!(
        error.contains("client/components/unreachable/card/tac.html"),
        "{error}"
    );
}

#[test]
fn component_style_ownership_export_only_component_cannot_bypass_guard() {
    let root = tempfile::tempdir().expect("export only source");
    write(
        root.path(),
        "client/manifest.json",
        r#"{"tac":{"elements":["card-view"]}}"#,
    );
    write(
        root.path(),
        "client/components/card/view/tac.html",
        "<style>.probe{display:grid}</style>",
    );
    assert!(failure(root.path()).to_string().contains("style element"));
}

#[test]
fn component_style_ownership_native_build_reuses_same_component_diagnostic() {
    let root = project("<link rel=\"stylesheet\" href=\"/shared/hidden.css\"><p>Card</p>");
    write(
        root.path(),
        "client/shared/hidden.css",
        ".probe{display:grid}",
    );
    let options = NativeBuildOptions {
        package: false,
        ..NativeBuildOptions::default()
    };
    let error = NativeCompiler::build(root.path(), &options).expect_err("native source ownership");
    assert!(error.to_string().contains("TY1403"), "{error}");
    assert!(
        error
            .to_string()
            .contains("client/components/card/view/tac.html"),
        "{error}"
    );
}

#[test]
fn component_style_ownership_inert_style_strings_comments_and_inline_attributes_remain() {
    accepted(
        "<!-- <style>.probe{display:grid}</style> --><pre><code>&lt;link rel=\"stylesheet\" href=\"/shared/hidden.css\"&gt;</code></pre><p title=\"style stylesheet\" style=\"color:navy\">Card</p>",
    );
}

#[test]
fn component_style_ownership_icon_href_binding_is_not_a_stylesheet() {
    let root = resource_project("<link rel=\"icon\" :href=\"resource.url\"><p>Card</p>");
    assert!(WebCompiler::build(root.path(), &BuildOptions::default()).is_ok());
}

#[test]
fn component_style_ownership_image_preload_href_binding_is_not_a_stylesheet() {
    let root =
        resource_project("<link rel=\"preload\" as=\"image\" :href=\"resource.url\"><p>Card</p>");
    assert!(WebCompiler::build(root.path(), &BuildOptions::default()).is_ok());
}

#[test]
fn component_style_ownership_script_preload_is_not_a_stylesheet() {
    accepted("<link rel=\"preload\" as=\"script\" href=\"/shared/resource.js\"><p>Card</p>");
}

#[test]
fn component_style_ownership_literal_dynamic_non_style_role_is_safe() {
    accepted(
        "<link :rel=\"'icon'\" :href=\"'/shared/favicon.svg'\"><link rel=\"preload\" :as=\"'image'\" :href=\"'/shared/image.png'\"><p>Card</p>",
    );
}

#[test]
fn component_style_ownership_dynamic_as_on_static_icon_cannot_make_css_role() {
    let root = resource_project(
        "<link rel=\"icon\" :as=\"resource.kind\" :href=\"resource.url\"><p>Card</p>",
    );
    assert!(WebCompiler::build(root.path(), &BuildOptions::default()).is_ok());
}

#[test]
fn component_style_ownership_generic_frontend_retains_its_literal_style_contract() {
    let parsed = TemplateFrontend::compile(
        "<style>p[title=\"{name}\"]{color:navy}</style>",
        "generic/tac.html",
        &BTreeSet::new(),
    );
    assert!(parsed.is_ok(), "{parsed:?}");
}

#[test]
fn component_style_ownership_real_component_css_is_still_scoped() {
    let root = project("<p class=\"probe\">Card</p>");
    write(
        root.path(),
        "client/components/card/view/tac.css",
        ".probe{display:grid}",
    );
    let result = WebCompiler::build(root.path(), &BuildOptions::default())
        .expect("owned component stylesheet");
    let css = fs::read_to_string(result.output_directory().join(".tachyon/components.css"))
        .expect("scoped CSS");
    assert!(
        css.contains("@scope (tachyon-component[data-tac-scope=\"card-view\"])"),
        "{css}"
    );
    assert!(css.contains(".probe{display:grid}"), "{css}");
}

#[test]
fn component_style_ownership_registry_reports_without_route_reach() {
    let root = tempfile::tempdir().expect("unreached registry");
    write(
        root.path(),
        "client/components/card/view/tac.html",
        "<link rel=\"stylesheet\" href=\"/shared/hidden.css\">",
    );
    let error = ComponentRegistry::discover(root.path()).expect_err("registry style ownership");
    assert!(error.to_string().contains("TY1403"), "{error}");
}
