#![allow(clippy::expect_used)]

use std::path::Path;

#[test]
#[ignore = "requires the installed JDK; uses the supervised native Test runner"]
fn java_preference_strings_use_per_construction_typed_fakes_without_browser_access() {
    let root = tempfile::tempdir().expect("project");
    let files = [
        (
            "client/services/ChoiceService.java",
            "@Service\nclass ChoiceService {\n @Needs\n private final PreferenceStrings preferences;\n String read() { return preferences.read(\"tachyon.platform\"); }\n boolean remember(String platform) { return preferences.write(\"tachyon.platform\", platform); }\n}\n",
        ),
        (
            "client/tests/PreferenceTestSetup.java",
            "@TestSetup\nclass PreferenceTestSetup {\n static Behaviors.Configuration configure(Behaviors.Execution execution) { return new Behaviors.Configuration(java.util.Map.of(), java.util.Map.of(), java.util.Map.of(), null, null); }\n}\n",
        ),
        (
            "client/tests/ChoiceServiceTest.java",
            r#"@Test(PreferenceTestSetup)
class ChoiceServiceTest {
    @Fake
    PreferenceStrings preferences;

    void testRawStringsAndCapturedFakeIdentity() {
        java.util.Map<String, String> firstStore = new java.util.HashMap<>();
        preferences = new PreferenceStrings() {
            public String read(String key) { return firstStore.get(key); }
            public boolean write(String key, String value) { firstStore.put(key, value); return true; }
        };
        ChoiceService first = construct(ChoiceService.class);
        Assert.equals(true, first.remember("macOS"), "writes the raw value");
        Assert.equals("macOS", firstStore.get("tachyon.platform"), "uses the existing raw key without a prefix");
        preferences = new PreferenceStrings() {
            public String read(String key) { return null; }
            public boolean write(String key, String value) { return false; }
        };
        ChoiceService denied = construct(ChoiceService.class);
        Assert.equals(null, denied.read(), "denied or missing storage reads null");
        Assert.equals(false, denied.remember("Linux"), "denied storage writes false");
        Assert.equals("macOS", first.read(), "the original service retains its own fake");
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
    let report = super::run(
        root.path(),
        Path::new("client/tests/ChoiceServiceTest.java"),
    )
    .expect("supervised Java Test");
    assert_eq!(report.cases.len(), 1);
    assert!(report.cases[0].passed, "{report:?}");
}
