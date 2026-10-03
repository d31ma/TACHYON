use crate::ProjectDiscovery;
use crate::annotation_layout::UtilityLayout;
use crate::compiler::utility_module::UtilityModule;
use std::path::Path;

struct TacSourceConventionTests;

impl TacSourceConventionTests {
    const JAVASCRIPT: &'static str = "class TitleUtils { static title() { return 'Ready'; } }";
    const TYPESCRIPT: &'static str =
        "class TitleUtils { static title(): string { return 'Ready'; } }";
    const RUST: &'static str = "pub struct TitleUtils;\nimpl TitleUtils { const TITLE: &'static str = \"Ready\"; pub fn title() -> String { Self::TITLE.to_owned() } }";

    fn valid(path: &str, source: &str) {
        assert!(UtilityLayout::is_utility(Path::new(path)));
        UtilityLayout::check(Path::new(path), source)
            .unwrap_or_else(|failure| panic!("valid shared utility: {failure}"));
    }

    fn refused(path: &str, source: &str, code: u16) {
        let failure = UtilityLayout::check(Path::new(path), source)
            .err()
            .unwrap_or_else(|| panic!("invalid shared utility must be refused"));
        assert!(
            failure
                .diagnostics()
                .iter()
                .any(|found| found.code.number() == code)
        );
        assert_eq!(failure.diagnostics()[0].spans[0].file, path);
    }

    fn write(root: &Path, path: &str, source: &str) {
        let file = root.join(path);
        std::fs::create_dir_all(file.parent().unwrap_or_else(|| panic!("parent")))
            .unwrap_or_else(|failure| panic!("folder: {failure}"));
        std::fs::write(file, source).unwrap_or_else(|failure| panic!("source: {failure}"));
    }

    fn project() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap_or_else(|failure| panic!("project: {failure}"));
        Self::write(root.path(), "client/pages/tac.html", "<card-view />");
        Self::write(
            root.path(),
            "client/components/card/view/tac.html",
            "<section aria-label=\"Card\"><p>Card</p></section>",
        );
        root
    }

    fn no_page_css(path: &str) {
        let root = Self::project();
        Self::write(root.path(), path, ".card { margin: 0; }");
        let failure = ProjectDiscovery::discover(root.path())
            .err()
            .unwrap_or_else(|| panic!("page CSS must be refused"));
        let found = failure
            .diagnostics()
            .iter()
            .find(|found| {
                found.code.number() == 1309 && found.spans.iter().any(|span| span.file == path)
            })
            .unwrap_or_else(|| panic!("page CSS diagnostic must name its actual path: {failure}"));
        assert!(format!("{found:?}").contains("component"));
    }
}

#[test]
fn tac_source_convention_nested_javascript_shared_utility_has_the_existing_stateless_shape() {
    TacSourceConventionTests::valid(
        "client/shared/utils/text/TitleUtils.js",
        TacSourceConventionTests::JAVASCRIPT,
    );
}

#[test]
fn tac_source_convention_nested_typescript_shared_utility_has_the_existing_stateless_shape() {
    TacSourceConventionTests::valid(
        "client/shared/utils/text/TitleUtils.ts",
        TacSourceConventionTests::TYPESCRIPT,
    );
}

#[test]
fn tac_source_convention_nested_rust_shared_utility_keeps_associated_constants_and_snake_filename()
{
    TacSourceConventionTests::valid(
        "client/shared/utils/text/title_utils.rs",
        TacSourceConventionTests::RUST,
    );
}

#[test]
fn tac_source_convention_shared_utility_does_not_hide_a_service_marker() {
    TacSourceConventionTests::refused(
        "client/shared/utils/text/TitleUtils.js",
        "@Service\nclass TitleUtils {}",
        1310,
    );
}

#[test]
fn tac_source_convention_shared_utility_does_not_hide_a_helper_marker() {
    TacSourceConventionTests::refused(
        "client/shared/utils/text/TitleUtils.js",
        "@Helper\nclass TitleUtils {}",
        1310,
    );
}

#[test]
fn tac_source_convention_service_filename_is_not_a_shared_utility() {
    TacSourceConventionTests::refused(
        "client/shared/utils/text/CounterService.js",
        "class CounterService { static read() { return 'Ready'; } }",
        1310,
    );
}

#[test]
fn tac_source_convention_shared_utility_does_not_hold_mutable_state() {
    TacSourceConventionTests::refused(
        "client/shared/utils/text/TitleUtils.js",
        "class TitleUtils { static value = 1; static read() { return TitleUtils.value; } }",
        1316,
    );
}

#[test]
fn tac_source_convention_shared_utility_does_not_have_an_instance_method() {
    TacSourceConventionTests::refused(
        "client/shared/utils/text/TitleUtils.js",
        "class TitleUtils { read() { return 'Ready'; } }",
        1316,
    );
}

#[test]
fn tac_source_convention_shared_utility_module_publishes_its_validated_named_class() {
    let output = UtilityModule::exported(
        Path::new("client/shared/utils/text/TitleUtils.js"),
        TacSourceConventionTests::JAVASCRIPT,
    )
    .unwrap_or_else(|failure| panic!("shared utility export: {failure}"));
    assert_eq!(
        output,
        format!(
            "{}\nexport {{ TitleUtils }};\n",
            TacSourceConventionTests::JAVASCRIPT
        )
        .as_bytes()
    );
}

#[test]
fn tac_source_convention_legacy_client_utils_remain_supported() {
    TacSourceConventionTests::valid(
        "client/utils/text/TitleUtils.js",
        TacSourceConventionTests::JAVASCRIPT,
    );
}

#[test]
fn tac_source_convention_server_utils_contract_stays_unchanged() {
    TacSourceConventionTests::valid(
        "server/utils/text/TitleUtils.ts",
        TacSourceConventionTests::TYPESCRIPT,
    );
}

#[test]
fn tac_source_convention_does_not_reinterpret_server_shared_as_a_new_utility_root() {
    assert!(!UtilityLayout::is_utility(Path::new(
        "server/shared/utils/text/TitleUtils.js"
    )));
}

#[test]
fn tac_source_convention_root_page_cannot_own_a_named_stylesheet() {
    TacSourceConventionTests::no_page_css("client/pages/layout.css");
}

#[test]
fn tac_source_convention_nested_page_cannot_own_tac_css() {
    TacSourceConventionTests::no_page_css("client/pages/docs/deep/tac.css");
}

#[test]
fn tac_source_convention_nested_page_cannot_own_a_named_stylesheet() {
    TacSourceConventionTests::no_page_css("client/pages/docs/deep/layout.css");
}

#[test]
fn tac_source_convention_nested_java_shared_utility_preserves_its_existing_source_contract() {
    TacSourceConventionTests::valid(
        "client/shared/utils/text/TitleUtils.java",
        "final class TitleUtils { static String title() { return \"Ready\"; } }",
    );
}

#[test]
fn tac_source_convention_nested_csharp_shared_utility_preserves_its_existing_source_contract() {
    TacSourceConventionTests::valid(
        "client/shared/utils/text/TitleUtils.cs",
        "public static class TitleUtils { public static string Title() => \"Ready\"; }",
    );
}

#[test]
fn tac_source_convention_nested_dart_shared_utility_preserves_its_existing_source_contract() {
    TacSourceConventionTests::valid(
        "client/shared/utils/text/TitleUtils.dart",
        "class TitleUtils { static String title() => \"Ready\"; }",
    );
}

#[test]
fn tac_source_convention_nested_kotlin_shared_utility_preserves_its_existing_source_contract() {
    TacSourceConventionTests::valid(
        "client/shared/utils/text/TitleUtils.kt",
        "object TitleUtils { fun title(): String = \"Ready\" }",
    );
}

#[test]
fn tac_source_convention_nested_swift_shared_utility_preserves_its_existing_source_contract() {
    TacSourceConventionTests::valid(
        "client/shared/utils/text/TitleUtils.swift",
        "struct TitleUtils { static func title() -> String { return \"Ready\" } }",
    );
}

#[test]
fn tac_source_convention_nested_php_shared_utility_preserves_its_existing_source_contract() {
    TacSourceConventionTests::valid(
        "client/shared/utils/text/TitleUtils.php",
        "<?php\nclass TitleUtils { public static function title(): string { return \"Ready\"; } }",
    );
}

#[test]
fn tac_source_convention_nested_python_shared_utility_preserves_its_existing_source_contract() {
    TacSourceConventionTests::valid(
        "client/shared/utils/text/TitleUtils.py",
        "class TitleUtils:\n    @staticmethod\n    def title() -> str:\n        return \"Ready\"\n",
    );
}

#[test]
fn tac_source_convention_nested_ruby_shared_utility_preserves_its_existing_source_contract() {
    TacSourceConventionTests::valid(
        "client/shared/utils/text/TitleUtils.rb",
        "class TitleUtils\n  def self.title\n    \"Ready\"\n  end\nend\n",
    );
}

#[test]
fn tac_source_convention_nested_go_shared_utility_preserves_its_existing_source_contract() {
    TacSourceConventionTests::valid(
        "client/shared/utils/text/TitleUtils.go",
        "package main\n\ntype TitleUtils struct{}\n\nfunc (TitleUtils) Title() string { return \"Ready\" }\n",
    );
}

#[test]
fn tac_source_convention_same_language_shared_utilities_participate_in_existing_realm_composition()
{
    let root = TacSourceConventionTests::project();
    TacSourceConventionTests::write(
        root.path(),
        "client/shared/utils/text/TitleUtils.js",
        TacSourceConventionTests::JAVASCRIPT,
    );
    TacSourceConventionTests::write(root.path(), "client/services/TitleService.js", "");
    let source = crate::annotation_modules::assemble(
        root.path(),
        Path::new("client/services/TitleService.js"),
        "",
        "tac",
    )
    .unwrap_or_else(|failure| panic!("shared utility composition: {failure}"))
    .unwrap_or_else(|| panic!("shared utility must compose"));
    assert!(source.contains("class TitleUtils"));
    assert!(source.contains("Ready"));
}
