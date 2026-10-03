#![allow(clippy::expect_used)]

use super::{BuildOptions, WebCompiler};
use crate::Failure;
use std::fs;
use std::path::Path;

fn write(root: &Path, name: &str, source: &str) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().expect("source parent")).expect("source directory");
    fs::write(path, source).expect("source file");
}

fn project(source: &str) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("component style boundary project");
    write(root.path(), "client/pages/tac.html", "<card-view />");
    write(
        root.path(),
        "client/components/card/view/tac.html",
        "<main class=\"card\">Card</main>",
    );
    write(root.path(), "client/components/card/view/tac.css", source);
    root
}

fn refused(source: &str) -> Failure {
    let root = project(source);
    let failure =
        WebCompiler::build(root.path(), &BuildOptions::default()).expect_err("scope boundary");
    let problem = failure
        .diagnostics()
        .iter()
        .find(|problem| problem.code.number() == 1403)
        .expect("component CSS diagnostic");
    assert_eq!(problem.spans[0].file, "client/components/card/view/tac.css");
    assert_eq!(problem.spans[0].start, 0);
    assert_eq!(problem.spans[0].end, source.len() as u64);
    assert!(
        problem
            .help
            .as_deref()
            .expect("migration help")
            .contains("owned @scope")
    );
    failure
}

fn emitted(source: &str) -> String {
    let root = project(source);
    let built =
        WebCompiler::build(root.path(), &BuildOptions::default()).expect("valid scoped stylesheet");
    let css = fs::read_to_string(built.output_directory().join(".tachyon/components.css"))
        .expect("emitted component stylesheet");
    assert!(
        css.starts_with("@scope (tachyon-component[data-tac-scope=\"card-view\"])"),
        "{css}"
    );
    assert!(css.contains(source.trim_end()), "{css}");
    css
}

fn foundation(source: &str) -> tempfile::TempDir {
    let root = project(".card{display:grid}");
    write(
        root.path(),
        "client/manifest.json",
        r#"{"tac":{"styles":["/shared/styles/site.css"]}}"#,
    );
    write(root.path(), "client/shared/styles/site.css", source);
    root
}

#[test]
fn component_style_boundary_refuses_injected_scope_closure_and_global_rule() {
    assert!(
        refused("}\nbody{display:grid}\n@scope(.never){")
            .to_string()
            .contains("unexpected closing block")
    );
}

#[test]
fn component_style_boundary_refuses_scope_closure_after_valid_component_rule() {
    assert!(
        refused(".card{color:navy}}\nbody{display:grid}")
            .to_string()
            .contains("unexpected closing block")
    );
}

#[test]
fn component_style_boundary_refuses_unclosed_block_before_wrapping() {
    assert!(
        refused(".card{display:grid")
            .to_string()
            .contains("unclosed block")
    );
}

#[test]
fn component_style_boundary_refuses_unclosed_comment_before_wrapping() {
    assert!(
        refused(".card{display:grid}/*")
            .to_string()
            .contains("unclosed comment")
    );
}

#[test]
fn component_style_boundary_refuses_unclosed_string_before_wrapping() {
    assert!(
        refused(".card{content:\"}")
            .to_string()
            .contains("unclosed string")
    );
}

#[test]
fn component_style_boundary_valid_layout_nested_rules_and_at_rules_stay_scoped() {
    let css = emitted(
        "@layer cards{.card{display:grid;grid-template-columns:1fr 2fr;&:hover{transform:translateY(-1px)}}}@media(min-width:40rem){.card{gap:calc(1rem + 2px)}}@keyframes pulse{from{opacity:0}to{opacity:1}}",
    );
    assert!(css.contains("grid-template-columns:1fr 2fr"));
    assert!(css.contains("@keyframes pulse"));
}

#[test]
fn component_style_boundary_inert_delimiters_in_strings_and_comments_stay_scoped() {
    assert!(emitted("/* } body{display:grid} */.card::before{content:\"}; @scope(.never){\";font-family:\"brace } family\"}").contains(".card::before"));
}

#[test]
fn component_style_boundary_valid_unquoted_data_url_brace_is_not_a_css_block() {
    assert!(emitted(".card{background-image:url(data:image/svg+xml,%3Csvg%20xmlns=%22http://www.w3.org/2000/svg%22%3E%3Ctext%3E{%3C/text%3E%3C/svg%3E);display:grid}").contains("background-image:url(data:image/svg+xml,"));
}

#[test]
fn component_style_boundary_valid_quoted_url_brace_remains_literal() {
    assert!(
        emitted(".card{background-image:url(\"data:image/svg+xml,<svg>{</svg>\");display:grid}")
            .contains("url(\"data:image/svg+xml,")
    );
}

#[test]
fn component_style_boundary_valid_escaped_url_brace_and_identifier_stay_scoped() {
    assert!(
        emitted(r".card{background-image:u\72 l(data:image/svg+xml,\{);display:grid}")
            .contains(r"u\72 l(")
    );
}

#[test]
fn component_style_boundary_url_brace_cannot_offset_real_scope_closure() {
    assert!(
        refused(".card{background:url(data:,{)} }\nbody{display:grid}\n@scope(.never){}")
            .to_string()
            .contains("unexpected closing block")
    );
}

#[test]
fn component_style_boundary_uppercase_url_cannot_offset_real_scope_closure() {
    assert!(
        refused(".card{background:URL(data:,{)} }\nbody{display:grid}")
            .to_string()
            .contains("unexpected closing block")
    );
}

#[test]
fn component_style_boundary_escaped_url_name_cannot_offset_real_scope_closure() {
    assert!(
        refused(r".card{background:u\72l(data:,{)} } body{display:grid}")
            .to_string()
            .contains("unexpected closing block")
    );
}

#[test]
fn component_style_boundary_escape_terminator_space_cannot_hide_url_name() {
    assert!(
        refused(r".card{background:u\72 l(data:,{)} } body{display:grid}")
            .to_string()
            .contains("unexpected closing block")
    );
}

#[test]
fn component_style_boundary_refuses_unclosed_block_after_valid_url() {
    assert!(
        refused(".card{background:url(data:,{)")
            .to_string()
            .contains("unclosed block")
    );
}

#[test]
fn component_style_boundary_refuses_unclosed_url_before_wrapping() {
    assert!(
        refused(".card{background:url(data:,")
            .to_string()
            .contains("unclosed URL")
    );
}

#[test]
fn component_style_boundary_atomic_url_does_not_bypass_foundation_asset_guard() {
    let root = foundation("body{font-family:URL(data:,theme)}");
    let error = WebCompiler::build(root.path(), &BuildOptions::default())
        .expect_err("foundation asset URL");
    assert!(
        error
            .to_string()
            .contains("Foundation values cannot load assets"),
        "{error}"
    );
    assert!(
        error.to_string().contains("client/shared/styles/site.css"),
        "{error}"
    );
}

#[test]
fn component_style_boundary_quoted_local_import_contract_remains_valid() {
    let root = foundation("@import url(\"theme.css\");body{color:navy}");
    write(
        root.path(),
        "client/shared/styles/theme.css",
        ":root{--surface:navy}",
    );
    let built =
        WebCompiler::build(root.path(), &BuildOptions::default()).expect("quoted local import");
    assert_eq!(
        fs::read_to_string(built.output_directory().join("shared/styles/theme.css"))
            .expect("published imported stylesheet"),
        ":root{--surface:navy}"
    );
}

#[test]
fn component_style_boundary_unquoted_import_remains_a_located_refusal() {
    let root = foundation("@import url(theme.css);body{color:navy}");
    write(
        root.path(),
        "client/shared/styles/theme.css",
        ":root{--surface:navy}",
    );
    let error =
        WebCompiler::build(root.path(), &BuildOptions::default()).expect_err("unquoted import");
    assert!(
        error.to_string().contains("unescaped quoted local paths"),
        "{error}"
    );
    assert!(
        error.to_string().contains("client/shared/styles/site.css"),
        "{error}"
    );
}

#[test]
fn component_style_boundary_remote_quoted_import_stays_refused() {
    let root = foundation("@import url(\"https://example.test/theme.css\");body{color:navy}");
    let error =
        WebCompiler::build(root.path(), &BuildOptions::default()).expect_err("remote import");
    assert!(error.to_string().contains("TY1403"), "{error}");
    assert!(
        error.to_string().contains("client/shared/styles/site.css"),
        "{error}"
    );
}

#[test]
fn component_style_boundary_escaped_url_import_stays_refused() {
    let root = foundation(r#"@import u\72l("theme.css");body{color:navy}"#);
    let error =
        WebCompiler::build(root.path(), &BuildOptions::default()).expect_err("escaped import");
    assert!(error.to_string().contains("TY1403"), "{error}");
    assert!(
        error.to_string().contains("client/shared/styles/site.css"),
        "{error}"
    );
}

#[test]
fn component_style_boundary_delimiter_prefix_cannot_hide_url_scope_offset() {
    assert!(
        refused(".card{background:+url(data:,{)} }body{display:grid}")
            .to_string()
            .contains("unexpected closing block")
    );
}

#[test]
fn component_style_boundary_invalid_identifier_escape_cannot_hide_url_scope_offset() {
    assert!(
        refused(".card{background:prefix\\\nurl(data:,{)} }body{display:grid}")
            .to_string()
            .contains("unexpected closing block")
    );
}

#[test]
fn component_style_boundary_valid_crlf_string_continuation_stays_scoped() {
    assert!(emitted(".card::before{content:\"brace\\\r\n} literal\"}").contains(".card::before"));
}

#[test]
fn component_style_boundary_unescaped_form_feed_cannot_hide_scope_closure_in_string() {
    assert!(
        refused(".card{content:\"prefix\x0c} }body{display:grid}\"}")
            .to_string()
            .contains("unescaped newline in a string")
    );
}
