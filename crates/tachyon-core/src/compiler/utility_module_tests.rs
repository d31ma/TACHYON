#![allow(clippy::expect_used)]

use super::utility_module::UtilityModule;
use std::path::Path;

struct UtilityModuleFixture;

impl UtilityModuleFixture {
    const PATH: &'static str = "client/utils/TitleUtils.js";
    const BODY: &'static str = "class TitleUtils { static location() { return new URL('../shared/assets/item.txt', import.meta.url).href; } }";
}

#[test]
fn utility_module_recognizes_code_meta_without_rewriting_comments_or_literals() {
    let path = Path::new(UtilityModuleFixture::PATH);
    for source in [
        UtilityModuleFixture::BODY,
        "class TitleUtils { static location() { return import /* note */ . meta.url; } }",
    ] {
        assert!(UtilityModule::needs_module(path, source).expect("mask"));
    }
    for source in [
        "class TitleUtils { static location() { return 'import.meta.url'; } }",
        "// import.meta.url\nclass TitleUtils { static location() { return 'Ready'; } }",
    ] {
        assert!(!UtilityModule::needs_module(path, source).expect("mask"));
    }
}

#[test]
fn utility_module_preserves_named_default_bare_and_alias_exports() {
    let path = Path::new(UtilityModuleFixture::PATH);
    for source in [
        format!("export {}", UtilityModuleFixture::BODY),
        format!("{}\nexport {{ TitleUtils }};", UtilityModuleFixture::BODY),
        format!(
            "{}\nexport {{ TitleUtils as TitleUtils }};",
            UtilityModuleFixture::BODY
        ),
    ] {
        assert_eq!(
            UtilityModule::exported(path, &source).expect("export"),
            source.as_bytes()
        );
    }
    for source in [
        UtilityModuleFixture::BODY.to_owned(),
        format!("export default {}", UtilityModuleFixture::BODY),
        format!(
            "{}\nexport {{ TitleUtils as Label }};",
            UtilityModuleFixture::BODY
        ),
    ] {
        let emitted = UtilityModule::exported(path, &source).expect("export");
        assert_eq!(
            String::from_utf8(emitted).expect("UTF-8"),
            format!("{source}\nexport {{ TitleUtils }};\n")
        );
    }
}

#[test]
fn utility_module_does_not_publish_an_unvalidated_stateful_utility() {
    let source =
        "class TitleUtils { static value = 1; static text() { return String(TitleUtils.value); } }";
    let failure = UtilityModule::exported(Path::new(UtilityModuleFixture::PATH), source)
        .expect_err("state stays rejected");
    assert_eq!(failure.diagnostics()[0].code.number(), 1316);
}
