#![allow(clippy::expect_used)]

use std::path::Path;

struct JavaTestSettingsProject;

impl JavaTestSettingsProject {
    const SETUP: (&'static str, &'static str) = (
        "client/tests/LabelTestSetup.java",
        "@TestSetup\n@WhenSetting({\"name\":\"TAC_LABEL_ENABLED\",\"havingValue\":\"true\",\"matchIfMissing\":false})\nclass LabelTestSetup {\n    static Behaviors.Configuration configure(Behaviors.Execution execution) {\n        return new Behaviors.Configuration(java.util.Map.of(), java.util.Map.of(), java.util.Map.of(), (call, current) -> true);\n    }\n}\n",
    );
    const SERVICE: (&'static str, &'static str) = (
        "client/services/LabelService.java",
        "@Service\nclass LabelService {\n    String label = \"real\";\n}\n",
    );
    const UNIT: &'static str = "client/tests/LabelServiceTest.java";

    fn write(root: &Path, settings: &str) {
        let unit = format!(
            "@Test(LabelTestSetup.class, {{\"partition\":\"java-settings-tests\"}})\n{settings}class LabelServiceTest {{\n    void testTheSetupIsActive() {{\n        Assert.equals(\"real\", new LabelService().label);\n    }}\n}}\n"
        );
        for (relative, source) in [Self::SETUP, Self::SERVICE, (Self::UNIT, unit.as_str())] {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().expect("fixture parent"))
                .expect("fixture directory");
            std::fs::write(path, source).expect("fixture source");
        }
    }

    fn report(settings: &str) -> Option<Result<super::TestReport, crate::Failure>> {
        if std::process::Command::new("javac")
            .arg("-version")
            .output()
            .is_err()
        {
            assert!(std::env::var_os("CI").is_none(), "CI must provide javac");
            eprintln!("skipping: javac is unavailable");
            return None;
        }
        let root = tempfile::tempdir().expect("test project");
        Self::write(root.path(), settings);
        Some(super::run(root.path(), Path::new(Self::UNIT)))
    }
}

#[test]
fn java_test_settings_reach_the_test_setup() {
    let Some(report) = JavaTestSettingsProject::report(
        "@TestSettings({\"tac.label.enabled\":\"true\",\"tac.label.count\":3,\"tac.label.shown\":true})\n",
    ) else {
        return;
    };
    let cases = report.expect("Java test unit").cases;
    assert_eq!(cases.len(), 1, "{cases:?}");
    assert!(cases[0].passed, "{cases:?}");
}

#[test]
fn a_java_test_unit_without_the_setting_leaves_the_setup_inactive() {
    let Some(report) = JavaTestSettingsProject::report("") else {
        return;
    };
    let failure = report.expect_err("inactive setup");
    assert!(
        format!("{failure:?}").contains("inactive-configuration"),
        "{failure:?}"
    );
}
