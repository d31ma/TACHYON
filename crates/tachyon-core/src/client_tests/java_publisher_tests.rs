#![allow(clippy::expect_used)]

use std::path::Path;

#[test]
#[ignore = "requires the installed JDK; validates per-construction typed publisher/preference Test fakes"]
fn java_injected_publisher_tests_preserve_retention_and_independent_fake_owners() {
    let root = tempfile::tempdir().expect("project");
    let files = [
        (
            "client/services/ChoiceService.java",
            "@Service\nclass ChoiceService {\n @Needs\n private final ApplicationEventPublisher events;\n @Needs\n private final PreferenceStrings preferences;\n String choose(String platform) { preferences.write(\"tachyon.platform\", platform); return events.publishEvent(platform, true); }\n}\n",
        ),
        (
            "client/tests/ChoiceTestSetup.java",
            "@TestSetup\nclass ChoiceTestSetup {\n static Behaviors.Configuration configure(Behaviors.Execution execution) { return new Behaviors.Configuration(java.util.Map.of(), java.util.Map.of(), java.util.Map.of(), null, null); }\n}\n",
        ),
        (
            "client/tests/ChoiceServiceTest.java",
            r#"@Test(ChoiceTestSetup)
class ChoiceServiceTest {
    @Fake
    ApplicationEventPublisher events;
    @Fake
    PreferenceStrings preferences;

    void testRetainedPublicationEvenWhenStorageIsDenied() {
        java.util.List<Object> firstEvents = new java.util.ArrayList<>();
        events = new ApplicationEventPublisher() {
            @Override
            public <T> T publishEvent(T value, boolean retain) {
                firstEvents.add(value);
                firstEvents.add(retain);
                return value;
            }
        };
        preferences = new PreferenceStrings() {
            public String read(String key) { return null; }
            public boolean write(String key, String value) {
                Assert.equals("tachyon.platform", key, "unchanged raw key");
                return false;
            }
        };
        ChoiceService first = construct(ChoiceService.class);
        java.util.List<Object> secondEvents = new java.util.ArrayList<>();
        events = new ApplicationEventPublisher() {
            @Override
            public <T> T publishEvent(T value, boolean retain) {
                secondEvents.add(value);
                secondEvents.add(retain);
                return value;
            }
        };
        ChoiceService second = construct(ChoiceService.class);
        Assert.equals("macOS", first.choose("macOS"), "return value preserved");
        Assert.equals("Windows", second.choose("Windows"), "second owner works");
        Assert.equals(java.util.List.of("macOS", true), firstEvents, "first captured its original fake");
        Assert.equals(java.util.List.of("Windows", true), secondEvents, "second captured its own fake");
        events = null;
        boolean missing = false;
        try { construct(ChoiceService.class); } catch (IllegalStateException expected) { missing = true; }
        Assert.equals(true, missing, "missing publisher fake cannot fall back to a browser provider");
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
