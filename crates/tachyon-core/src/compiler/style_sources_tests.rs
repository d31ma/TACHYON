#![allow(clippy::expect_used)]

use super::{BuildOptions, WebCompiler};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

fn write_file(root: &Path, name: &str, source: &str) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().expect("source directory")).expect("create source directory");
    fs::write(path, source).expect("source file");
}

fn project(css: &str, declarations: Value) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("foundation project");
    write_file(root.path(), "client/pages/tac.html", "<foundation-probe />");
    write_file(
        root.path(),
        "client/components/foundation/probe/tac.html",
        "<main>Foundation</main>",
    );
    write_file(root.path(), "client/shared/styles/site.css", css);
    write_file(
        root.path(),
        "client/manifest.json",
        &json!({"tac":{"styles":["/shared/styles/site.css"],"styleSources":declarations}})
            .to_string(),
    );
    root
}

fn build(root: &Path) -> Result<super::BuildResult, String> {
    WebCompiler::build(root, &BuildOptions::default()).map_err(|failure| failure.to_string())
}

fn rejected(css: &str) -> String {
    let root = project(css, json!({}));
    let error = build(root.path()).expect_err("invalid global CSS");
    assert!(error.contains("TY1403"), "{error}");
    assert!(error.contains("client/shared/styles/site.css"), "{error}");
    error
}

fn vendor(css: &str) -> Value {
    json!({"kind":"vendor","package":"fixture-styles","version":"1.2.3","license":"MIT","sha256":super::hex_digest(Sha256::digest(css.as_bytes()))})
}

#[test]
fn style_sources_explicit_foundation_is_published_through_existing_links() {
    let css = ":root{--step-2xl:1.5rem;color-scheme:dark}body{margin:0;font-family:system-ui;background:#123456;color:#abcdef}*::before,*::after{box-sizing:border-box}";
    let root = project(
        css,
        json!({"/shared/styles/site.css":{"kind":"foundation"}}),
    );
    let built = build(root.path()).expect("foundation build");
    let html = fs::read_to_string(built.output_directory().join("index.html")).expect("page");
    assert!(html.contains("href=\"/shared/styles/site.css\""), "{html}");
    assert_eq!(
        fs::read_to_string(built.output_directory().join("shared/styles/site.css"))
            .expect("published CSS"),
        css
    );
}

#[test]
fn style_sources_legacy_strings_receive_same_foundation_audit() {
    let root = project("body{color:navy;font-size:1rem;line-height:1.5}", json!({}));
    assert!(build(root.path()).is_ok());
}

#[test]
fn style_sources_theme_and_layer_groups_remain_foundation() {
    let css = "@layer theme,reset;@layer theme{:root[w-theme=\"dark\"]{--surface:#112233}}@media(prefers-contrast:more){:root{color:white}}@supports(color:color-mix(in srgb,red,blue)){body{color:color-mix(in srgb,red,blue)}}";
    let root = project(css, json!({}));
    assert!(build(root.path()).is_ok());
}

#[test]
fn style_sources_narrow_reduced_motion_reset_is_allowed() {
    let root = project(
        "@media(prefers-reduced-motion:reduce){*{animation-duration:0.01ms;transition-duration:0s;animation-iteration-count:1}}",
        json!({}),
    );
    assert!(build(root.path()).is_ok());
}

#[test]
fn style_sources_registered_theme_property_preserves_inheritance_contract() {
    let root = project(
        "@property --tw-font-weight{syntax:\"*\";inherits:false}@layer properties{@supports(font-weight:400){*,::before,::after{--tw-font-weight:initial}}}",
        json!({}),
    );
    assert!(build(root.path()).is_ok());
}

#[test]
fn style_sources_property_registration_cannot_hide_application_layout() {
    assert!(
        rejected("@property --weight{syntax:\"*\";inherits:false;display:grid}")
            .contains("only syntax")
    );
}

#[test]
fn style_sources_application_class_is_refused_even_with_only_color() {
    assert!(rejected(".card{color:red}").contains("component HTML"));
}

#[test]
fn style_sources_body_layout_is_refused() {
    assert!(rejected("body{display:grid}").contains("layout/position/grid/flex"));
}

#[test]
fn style_sources_application_ids_are_refused() {
    assert!(rejected("#application{font-family:system-ui}").contains("selectors"));
}

#[test]
fn style_sources_descendant_selectors_are_not_plain_defaults() {
    assert!(rejected("body main{color:red}").contains("selectors"));
}

#[test]
fn style_sources_split_selector_tokens_cannot_masquerade_as_body() {
    assert!(rejected("bo dy{color:red}").contains("selectors"));
}

#[test]
fn style_sources_keyframes_are_refused_in_authored_foundation() {
    assert!(rejected("@keyframes spin{from{color:red}to{color:blue}}").contains("at-rule"));
}

#[test]
fn style_sources_comments_and_string_delimiters_are_not_rules() {
    let root = project(
        "/* .card{display:grid} */body{font-family:\"semi; colon } font\";color:navy}",
        json!({}),
    );
    assert!(build(root.path()).is_ok());
}

#[test]
fn style_sources_malformed_comment_is_refused() {
    assert!(rejected("body{color:navy}/* no close").contains("unclosed comment"));
}

#[test]
fn style_sources_pinned_vendor_bytes_allow_declared_vendor_layout() {
    let css = ".vendor-button{display:flex;gap:1rem}";
    let root = project(css, json!({"/shared/styles/site.css":vendor(css)}));
    assert!(build(root.path()).is_ok());
}

#[test]
fn style_sources_pinned_vendor_hash_mismatch_is_refused() {
    let root = project(
        ".vendor{display:grid}",
        json!({"/shared/styles/site.css":vendor(".different{display:grid}")}),
    );
    assert!(
        build(root.path())
            .expect_err("hash mismatch")
            .contains("do not match")
    );
}

#[test]
fn style_sources_vendor_directory_and_minified_name_are_not_exemptions() {
    let root = project("@import '../vendor/theme.min.css';", json!({}));
    write_file(
        root.path(),
        "client/shared/vendor/theme.min.css",
        ".card{display:grid}",
    );
    let error = build(root.path()).expect_err("undeclared vendor");
    assert!(
        error.contains("client/shared/vendor/theme.min.css"),
        "{error}"
    );
    assert!(error.contains("selectors"), "{error}");
}

#[test]
fn style_sources_nested_relative_foundation_imports_are_followed() {
    let root = project(
        "@import './nested/theme.css' layer(theme);body{margin:0}",
        json!({}),
    );
    write_file(
        root.path(),
        "client/shared/styles/nested/theme.css",
        "@import '../../colors.css';:root{--accent:blue}",
    );
    write_file(
        root.path(),
        "client/shared/colors.css",
        ":root{--surface:white}",
    );
    assert!(build(root.path()).is_ok());
}

#[test]
fn style_sources_import_cannot_hide_global_application_selector() {
    let root = project("@import './hidden.css';:root{--surface:white}", json!({}));
    write_file(
        root.path(),
        "client/shared/styles/hidden.css",
        ".page{display:flex}",
    );
    assert!(
        build(root.path())
            .expect_err("hidden app CSS")
            .contains("client/shared/styles/hidden.css")
    );
}

#[test]
fn style_sources_remote_import_is_refused() {
    assert!(rejected("@import 'https://example.invalid/theme.css';").contains("local"));
}

#[test]
fn style_sources_protocol_relative_import_is_refused() {
    assert!(rejected("@import '//shared/remote.css';").contains("protocol-relative"));
}

#[test]
fn style_sources_import_cannot_escape_shared_capability() {
    assert!(rejected("@import '../../../secret.css';").contains("escapes"));
}

#[test]
fn style_sources_import_cycle_is_refused() {
    let root = project("@import './nested.css';", json!({}));
    write_file(
        root.path(),
        "client/shared/styles/nested.css",
        "@import './site.css';",
    );
    assert!(build(root.path()).expect_err("cycle").contains("cycle"));
}

#[test]
fn style_sources_vendor_imports_still_require_bounded_graph() {
    let css = "@import 'https://example.invalid/vendor.css';.vendor{display:grid}";
    let root = project(css, json!({"/shared/styles/site.css":vendor(css)}));
    assert!(
        build(root.path())
            .expect_err("vendor remote import")
            .contains("local")
    );
}

#[test]
fn style_sources_escaped_vendor_at_rule_cannot_hide_import() {
    let css = "@im\\port 'https://example.invalid/vendor.css';.vendor{display:grid}";
    let root = project(css, json!({"/shared/styles/site.css":vendor(css)}));
    assert!(
        build(root.path())
            .expect_err("escaped vendor import")
            .contains("Escaped CSS")
    );
}

#[test]
fn style_sources_vendor_nested_import_is_refused() {
    let css = "@media all{@import './nested.css';}.vendor{display:grid}";
    let root = project(css, json!({"/shared/styles/site.css":vendor(css)}));
    assert!(
        build(root.path())
            .expect_err("nested vendor import")
            .contains("top-level")
    );
}

#[test]
fn style_sources_utf8_is_required() {
    let root = project("", json!({}));
    fs::write(root.path().join("client/shared/styles/site.css"), [255_u8])
        .expect("invalid CSS bytes");
    assert!(
        build(root.path())
            .expect_err("UTF-8 guard")
            .contains("UTF-8")
    );
}

#[test]
fn style_sources_source_size_is_bounded_before_publication() {
    let root = project(&" ".repeat(1_048_577), json!({}));
    assert!(
        build(root.path())
            .expect_err("source size")
            .contains("1 MiB")
    );
}

#[cfg(unix)]
#[test]
fn style_sources_symlinked_import_is_refused() {
    let root = project("@import './alias.css';", json!({}));
    write_file(
        root.path(),
        "client/shared/styles/target.css",
        ":root{--surface:white}",
    );
    std::os::unix::fs::symlink(
        "target.css",
        root.path().join("client/shared/styles/alias.css"),
    )
    .expect("CSS symlink");
    assert!(build(root.path()).is_err());
}

#[test]
fn style_sources_site_style_does_not_change_yon_only_project() {
    let root = tempfile::tempdir().expect("Yon-only project");
    write_file(
        root.path(),
        "server/controllers/PingController.js",
        "@RequestRoute(\"/ping\")\n@Public\nclass PingController { @GetRoute getPing() { return 'pong' } }",
    );
    assert!(build(root.path()).is_ok());
}

#[test]
fn style_sources_diagnostic_points_to_actual_offending_rule_offset() {
    let error = rejected("body{color:navy} .card{color:red}");
    assert!(error.contains("site.css:17..17"), "{error}");
}

#[test]
fn style_sources_vendor_version_cannot_be_a_floating_latest_tag() {
    let css = ".vendor{display:grid}";
    let mut declaration = vendor(css);
    declaration["version"] = json!("latest");
    let root = project(css, json!({"/shared/styles/site.css":declaration}));
    assert!(
        build(root.path())
            .expect_err("floating version")
            .contains("exact pinned version")
    );
}

fn imported_files(root: &Path, count: usize, bytes: usize) {
    let imports: String = (0..count)
        .map(|index| format!("@import './file-{index}.css';\n"))
        .collect();
    write_file(root, "client/shared/styles/site.css", &imports);
    let css = format!("body{{color:navy}}{}", " ".repeat(bytes));
    for index in 0..count {
        write_file(
            root,
            &format!("client/shared/styles/file-{index}.css"),
            &css,
        );
    }
}

fn import_chain(root: &Path, count: usize) {
    write_file(
        root,
        "client/shared/styles/site.css",
        "@import './file-0.css';",
    );
    for index in 0..count {
        write_file(
            root,
            &format!("client/shared/styles/file-{index}.css"),
            &format!("@import './file-{}.css';", index + 1),
        );
    }
    write_file(
        root,
        &format!("client/shared/styles/file-{count}.css"),
        "body{color:navy}",
    );
}

#[test]
fn style_sources_unique_import_file_count_is_bounded() {
    let root = project("", json!({}));
    imported_files(root.path(), 128, 0);
    assert!(
        build(root.path())
            .expect_err("file count bound")
            .contains("128 files")
    );
}

#[test]
fn style_sources_import_depth_is_bounded() {
    let root = project("", json!({}));
    import_chain(root.path(), 33);
    assert!(
        build(root.path())
            .expect_err("import depth bound")
            .contains("32 levels")
    );
}

#[test]
fn style_sources_total_import_bytes_are_bounded() {
    let root = project("", json!({}));
    imported_files(root.path(), 9, 1_000_000);
    assert!(
        build(root.path())
            .expect_err("total graph bytes")
            .contains("8 MiB")
    );
}

#[test]
fn style_sources_shared_import_reuse_is_not_a_cycle() {
    let root = project("@import './first.css';@import './second.css';", json!({}));
    write_file(
        root.path(),
        "client/shared/styles/first.css",
        "@import './common.css';body{color:navy}",
    );
    write_file(
        root.path(),
        "client/shared/styles/second.css",
        "@import './common.css';:root{--accent:blue}",
    );
    write_file(
        root.path(),
        "client/shared/styles/common.css",
        "body{margin:0}",
    );
    assert!(build(root.path()).is_ok());
}

#[test]
fn style_sources_url_import_follows_same_local_graph() {
    let root = project("@import url('./theme.css') layer(theme);", json!({}));
    write_file(
        root.path(),
        "client/shared/styles/theme.css",
        "body{color:navy}",
    );
    assert!(build(root.path()).is_ok());
}

#[test]
fn style_sources_uppercase_css_import_extension_remains_portable() {
    let root = project("@import './theme.CSS';", json!({}));
    write_file(
        root.path(),
        "client/shared/styles/theme.CSS",
        "body{color:navy}",
    );
    assert!(build(root.path()).is_ok());
}

#[test]
fn style_sources_background_position_cannot_masquerade_as_color() {
    assert!(rejected("body{background:red center}").contains("layout/position/grid/flex"));
}

#[test]
fn style_sources_background_color_function_cannot_hide_position_suffix() {
    assert!(rejected("body{background:rgb(0 0 0) center}").contains("layout/position/grid/flex"));
}

#[test]
fn style_sources_escaped_url_function_is_not_a_theme_variable() {
    assert!(
        rejected(":root{--asset:u\\72l('https://example.invalid/image.png')}")
            .contains("escape syntax")
    );
}

#[test]
fn style_sources_font_name_literal_does_not_create_an_asset_function() {
    let root = project(
        "body{font-family:\"url( ) !important { }\";color:navy}",
        json!({}),
    );
    assert!(build(root.path()).is_ok());
}

#[test]
fn style_sources_original_important_reduced_motion_reset_is_preserved() {
    let css = "@media (prefers-reduced-motion: reduce){*{animation-duration:0.01ms !important;transition-duration:0.01ms !important;animation-iteration-count:1 !important;scroll-behavior:auto !important}}";
    let root = project(css, json!({}));
    let built = build(root.path()).expect("original accessible reset");
    assert_eq!(
        fs::read_to_string(built.output_directory().join("shared/styles/site.css"))
            .expect("published reset"),
        css
    );
}

#[test]
fn style_sources_reduced_motion_priority_survives_layer_and_support_groups() {
    let root = project(
        "@layer reset{@media(prefers-reduced-motion:reduce){@supports(animation-duration:0s){*,*::before,*::after{animation-duration:0s!important;transition-duration:0ms ! important}}}}",
        json!({}),
    );
    assert!(build(root.path()).is_ok());
}

#[test]
fn style_sources_priority_without_reduced_motion_context_is_refused() {
    assert!(rejected("*{transition-duration:0.01ms!important}").contains("priority is limited"));
}

#[test]
fn style_sources_priority_in_other_media_is_refused() {
    assert!(
        rejected("@media(prefers-contrast:more){*{transition-duration:0s!important}}")
            .contains("priority is limited")
    );
}

#[test]
fn style_sources_media_alternative_cannot_broaden_reduced_motion_priority() {
    assert!(
        rejected("@media(prefers-reduced-motion:reduce),all{*{transition-duration:0s!important}}")
            .contains("priority is limited")
    );
}

#[test]
fn style_sources_negated_reduced_motion_cannot_enable_priority() {
    assert!(
        rejected("@media not (prefers-reduced-motion:reduce){*{transition-duration:0s!important}}")
            .contains("priority is limited")
    );
}

#[test]
fn style_sources_reduced_motion_context_does_not_enable_color_priority() {
    assert!(
        rejected("@media(prefers-reduced-motion:reduce){*{color:red!important}}")
            .contains("priority is limited")
    );
}

#[test]
fn style_sources_reduced_motion_context_does_not_enable_layout_priority() {
    assert!(
        rejected("@media(prefers-reduced-motion:reduce){*{display:grid!important}}")
            .contains("priority is limited")
    );
}

#[test]
fn style_sources_reduced_motion_priority_requires_universal_reset_selector() {
    assert!(
        rejected("@media(prefers-reduced-motion:reduce){body{transition-duration:0s!important}}")
            .contains("priority is limited")
    );
}

#[test]
fn style_sources_reduced_motion_priority_rejects_arbitrary_duration() {
    assert!(
        rejected("@media(prefers-reduced-motion:reduce){*{transition-duration:3s!important}}")
            .contains("priority is limited")
    );
}

#[test]
fn style_sources_reduced_motion_priority_rejects_variable_duration() {
    assert!(rejected("@media(prefers-reduced-motion:reduce){*{transition-duration:var(--duration)!important}}").contains("priority is limited"));
}

#[test]
fn style_sources_reduced_motion_priority_rejects_repeated_animations() {
    assert!(
        rejected("@media(prefers-reduced-motion:reduce){*{animation-iteration-count:2!important}}")
            .contains("priority is limited")
    );
}

#[test]
fn style_sources_reduced_motion_priority_rejects_smooth_scrolling() {
    assert!(
        rejected("@media(prefers-reduced-motion:reduce){*{scroll-behavior:smooth!important}}")
            .contains("priority is limited")
    );
}

#[test]
fn style_sources_reduced_motion_priority_does_not_enable_zero_layout() {
    assert!(
        rejected("@media(prefers-reduced-motion:reduce){*{margin:0!important}}")
            .contains("priority is limited")
    );
}
