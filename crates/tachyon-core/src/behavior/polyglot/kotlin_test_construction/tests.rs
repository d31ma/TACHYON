#![allow(clippy::expect_used)]

use std::path::Path;

fn lowered(fake: &str) -> String {
    let source = format!(
        "@Test(TitleTestSetup)\nclass TitleServiceTest {{\n {fake}\n fun testWrite() {{}}\n}}\n\
         @TestSetup\nclass TitleTestSetup {{\n companion object {{ fun configure(execution: Behaviors.Execution): Behaviors.Configuration = Behaviors.Configuration() }}\n}}\n\
         @Service\nclass TitleService {{\n @Needs\n val documentTitle: DocumentTitle\n fun write(title: String) {{ documentTitle.set(title) }}\n}}\n"
    );
    let path = Path::new("client/tests/TitleServiceTest.kt");
    let source = crate::behavior::ServiceRoles::read(path, &source)
        .expect("role stripping")
        .source;
    crate::behavior::lower(path, &source, "tac")
        .expect("Test lowering")
        .expect("source")
}

#[test]
fn kotlin_test_document_title_construct_uses_an_assigned_typed_fake() {
    let source = lowered("@Fake\n var documentTitle: DocumentTitle? = null");
    assert!(source.contains("return type.java.cast(TitleService(checkNotNull(this.documentTitle)"));
    assert!(source.contains("Test.construct needs an assigned Fake named documentTitle"));
    assert!(source.contains("Test.construct needs an active test setup"));
    assert!(!source.contains("BrowserDocumentTitle"));
}

#[test]
fn kotlin_test_document_title_construct_refuses_unmarked_and_wrong_typed_values() {
    for field in [
        "var documentTitle: DocumentTitle? = null",
        "@Fake\n var documentTitle: String? = null",
    ] {
        let source = lowered(field);
        assert!(source.contains("matching typed Fake named documentTitle for TitleService"));
        assert!(!source.contains("return type.java.cast(TitleService("));
    }
}
