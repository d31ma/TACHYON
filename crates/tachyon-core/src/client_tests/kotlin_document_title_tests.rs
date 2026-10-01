#![allow(clippy::expect_used)]

use std::path::Path;

#[test]
#[ignore = "requires installed Kotlin/JVM; supervised native Test runner"]
fn kotlin_document_title_tests_capture_typed_fakes_per_construction() {
    let root = tempfile::tempdir().expect("project");
    let files = [
        (
            "client/services/TitleService.kt",
            "@Service\nclass TitleService {\n @Needs\n val documentTitle: DocumentTitle\n fun write(title: String) { documentTitle.set(title) }\n}\n",
        ),
        (
            "client/tests/TitleTestSetup.kt",
            "@TestSetup\nclass TitleTestSetup {\n companion object { fun configure(execution: Behaviors.Execution): Behaviors.Configuration = Behaviors.Configuration() }\n}\n",
        ),
        (
            "client/tests/TitleServiceTest.kt",
            r#"@Test(TitleTestSetup)
class TitleServiceTest {
    @Fake
    var documentTitle: DocumentTitle? = null

    fun testIndependentConstruction() {
        val firstWrites = mutableListOf<String>()
        documentTitle = object : DocumentTitle { override fun set(title: String) { firstWrites.add(title) } }
        val first = construct(TitleService::class)
        val secondWrites = mutableListOf<String>()
        documentTitle = object : DocumentTitle { override fun set(title: String) { secondWrites.add(title) } }
        val second = construct(TitleService::class)
        first.write("first — title")
        second.write("")
        Assert.equals(listOf("first — title"), firstWrites, "original fake retained")
        Assert.equals(listOf(""), secondWrites, "second owns a distinct fake")
    }

    fun testMissingFakeRefused() {
        documentTitle = null
        var refused = false
        try { construct(TitleService::class) }
        catch (error: IllegalStateException) { refused = error.message?.contains("assigned Fake named documentTitle") == true }
        Assert.equals(true, refused, "no real browser fallback")
    }
}
"#,
        ),
    ];
    for (relative, source) in files {
        let path = root.path().join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(path, source).expect("source");
    }
    let report = super::run(root.path(), Path::new("client/tests/TitleServiceTest.kt"))
        .expect("actual Test report");
    assert_eq!(report.cases.len(), 2);
    for case in report.cases {
        assert!(case.passed, "{}: {:?}", case.name, case.error);
    }
}
