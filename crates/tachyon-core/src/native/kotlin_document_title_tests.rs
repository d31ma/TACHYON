#![allow(clippy::expect_used)]

use super::registry::NativeCompanionInput;
use crate::project::NativeCompanion;

#[test]
fn kotlin_document_title_is_owned_factory_input_without_a_page_property() {
    let root = tempfile::tempdir().expect("project");
    let path = root.path().join("client/services/TitleService.kt");
    std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    std::fs::write(&path, "@Service\n@NeedsAll\nclass TitleService {\n private val documentTitle: DocumentTitle\n val label: String\n fun write() { documentTitle.set(label) }\n}\n").expect("service");
    let companions = NativeCompanionInput::web_services(
        root.path(),
        NativeCompanion::Kotlin,
        &[(String::from("title"), String::from("TitleService"), path)],
    );
    let source = super::registry::source(&companions, NativeCompanion::Kotlin)
        .expect("registry")
        .expect("source");
    assert!(source.contains("this.documentTitle = tacPage.documentTitle"));
    assert!(source.contains("tacCreate(tacProps, tacPage)"));
    assert!(source.contains("this.label = tacRequiredValue"));
    assert!(!source.contains("tacRequiredValue(tacProps, \"documentTitle\")"));
    assert!(!source.contains("\"documentTitle\" to TacField"));
    assert!(source.contains("tacRequired: List<String> = listOf(\"*\")"));
}

#[test]
fn kotlin_document_title_explicit_needs_uses_the_same_captured_provider() {
    let root = tempfile::tempdir().expect("project");
    let path = root.path().join("client/services/TitleService.kt");
    std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    std::fs::write(&path, "@Service\nclass TitleService {\n @Needs\n private val documentTitle: DocumentTitle\n fun write(title: String) { documentTitle.set(title) }\n}\n").expect("service");
    let companions = NativeCompanionInput::web_services(
        root.path(),
        NativeCompanion::Kotlin,
        &[(String::from("title"), String::from("TitleService"), path)],
    );
    let source = super::registry::source(&companions, NativeCompanion::Kotlin)
        .expect("registry")
        .expect("source");
    assert!(source.contains("this.documentTitle = tacPage.documentTitle"));
    assert!(!source.contains("documentTitle\" to TacField"));
}
