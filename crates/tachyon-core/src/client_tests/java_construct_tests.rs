use std::path::Path;

#[test]
#[ignore = "requires the installed JDK; runs through the supervised native Test runner"]
fn native_java_tests_construct_with_typed_fakes_and_preserve_private_fields() {
    let root = tempfile::tempdir().expect("test project");
    let files = [
        (
            "client/shared/native/java/ReadPort.java",
            "interface ReadPort { String read(); }\n",
        ),
        (
            "client/services/ChoiceService.java",
            "@Service\nclass ChoiceService {\n    @Needs\n    private final ReadPort port;\n    String answer() { return port.read(); }\n}\n",
        ),
        (
            "client/tests/ChoiceTestSetup.java",
            "@TestSetup\nclass ChoiceTestSetup {\n    static Behaviors.Configuration configure(Behaviors.Execution execution) { return new Behaviors.Configuration(java.util.Map.of(), java.util.Map.of(), java.util.Map.of(), null, null); }\n}\n",
        ),
        (
            "client/tests/ChoiceServiceTest.java",
            r#"@Test(ChoiceTestSetup)
class ChoiceServiceTest {
    @Fake
    ReadPort port;

    void testPerConstructionFakeIdentity() {
        port = () -> "first";
        ChoiceService first = construct(ChoiceService.class);
        port = () -> "second";
        ChoiceService second = construct(ChoiceService.class);
        Assert.equals("first", first.answer(), "an existing instance retains its injected fake");
        Assert.equals("second", second.answer(), "the next construction uses this test's current fake");
    }

    void testMissingFakeIsRefused() {
        port = null;
        try { construct(ChoiceService.class); throw new AssertionError("missing fake was accepted"); }
        catch (IllegalStateException refused) {
            Assert.equals("Test.construct needs an assigned Fake named port.", refused.getMessage(), "missing assignment is explicit");
        }
    }

    void testUnsupportedTargetIsRefused() {
        try { construct(ReadPort.class); throw new AssertionError("interface was instantiated"); }
        catch (IllegalArgumentException refused) {
            Assert.equals("Test.construct needs a class with a declared Needs constructor.", refused.getMessage(), "unsupported target is explicit");
        }
    }

    void testDisposedSetupCannotConstruct() {
        stopBehaviors();
        try { construct(ChoiceService.class); throw new AssertionError("closed setup constructed a service"); }
        catch (IllegalStateException refused) {
            Assert.equals("Test.construct needs an active test setup.", refused.getMessage(), "construction cannot outlive its setup");
        }
    }
}
"#,
        ),
        (
            "client/tests/WrongFakeTest.java",
            r#"@Test(ChoiceTestSetup)
class WrongFakeTest {
    @Fake
    String port;
    void testIncompatibleFakeIsRefused() {
        port = "not a port";
        try { construct(ChoiceService.class); throw new AssertionError("wrong fake type was accepted"); }
        catch (IllegalStateException refused) {
            Assert.equals("Test.construct needs a matching typed Fake named port for ChoiceService.", refused.getMessage(), "wrong type is explicit");
        }
    }
}
"#,
        ),
    ];
    for (relative, source) in files {
        let path = root.path().join(relative);
        std::fs::create_dir_all(path.parent().expect("fixture parent")).expect("fixture directory");
        std::fs::write(path, source).expect("fixture source");
    }
    for (name, cases) in [("ChoiceServiceTest", 4), ("WrongFakeTest", 1)] {
        let relative = format!("client/tests/{name}.java");
        let report =
            super::run(root.path(), Path::new(&relative)).expect("supervised native Java Test");
        assert_eq!(report.cases.len(), cases, "{name}");
        assert!(report.cases.iter().all(|case| case.passed), "{report:?}");
    }
}
