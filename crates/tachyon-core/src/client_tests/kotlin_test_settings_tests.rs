#![allow(clippy::expect_used)]

use crate::one_test_per_case_tests::one_test_per_case;
use std::path::Path;

struct KotlinTestSettingsProject;

impl KotlinTestSettingsProject {
    const SETUP: &'static str = r#"@TestSetup
@WhenProfile("test")
@WhenSetting({"name":"__SETTING__","havingValue":"__VALUE__","matchIfMissing":false})
class __NAME__ {
    companion object {
        fun configure(execution: Behaviors.Execution): Behaviors.Configuration = Behaviors.Configuration()
    }
}
"#;
    const SETUPS: [(&'static str, &'static str, &'static str); 3] = [
        ("CacheTestSetup", "TAC_CATALOG_CACHE_ENABLED", "true"),
        ("PageTestSetup", "TAC_CATALOG_PAGE_SIZE", "5"),
        ("TitleTestSetup", "TAC_CATALOG_TITLE", "Test catalog"),
    ];
    const UNIT: &'static str = "@Test(__SETUP__::class)\n@UseProfiles(\"test\")\n__SETTINGS__\nclass __NAME__ {\n    fun testTheSetupIsActive() {}\n}\n";
    const ALL_SETTINGS: &'static str = r#"@TestSettings({"tac.catalog.page.size":5,"tac.catalog.cache.enabled":true,"tac.catalog.title":"Test catalog"})"#;
    const CACHE_OFF: &'static str = r#"@TestSettings({"tac.catalog.page.size":5,"tac.catalog.cache.enabled":false,"tac.catalog.title":"Test catalog"})"#;
    const UNITS: [(
        &'static str,
        &'static str,
        &'static str,
        Option<&'static str>,
    ); 5] = [
        (
            "CacheSettingsTest",
            "CacheTestSetup",
            Self::ALL_SETTINGS,
            None,
        ),
        (
            "PageSettingsTest",
            "PageTestSetup",
            Self::ALL_SETTINGS,
            None,
        ),
        (
            "TitleSettingsTest",
            "TitleTestSetup",
            Self::ALL_SETTINGS,
            None,
        ),
        (
            "CacheOffTest",
            "CacheTestSetup",
            Self::CACHE_OFF,
            Some("inactive-configuration"),
        ),
        (
            "NoSettingsTest",
            "CacheTestSetup",
            "",
            Some("inactive-configuration"),
        ),
    ];

    fn write(root: &Path, relative: &str, source: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("fixture parent")).expect("fixture directory");
        std::fs::write(path, source).expect("fixture source");
    }

    fn write_project(root: &Path) {
        for (name, setting, value) in Self::SETUPS {
            let source = Self::SETUP
                .replace("__NAME__", name)
                .replace("__SETTING__", setting)
                .replace("__VALUE__", value);
            Self::write(root, &format!("client/tests/{name}.kt"), &source);
        }
        for (name, setup, settings, _) in Self::UNITS {
            let source = Self::UNIT
                .replace("__SETUP__", setup)
                .replace("__SETTINGS__", settings)
                .replace("__NAME__", name);
            Self::write(root, &format!("client/tests/{name}.kt"), &source);
        }
    }

    fn kotlinc_missing() -> bool {
        let missing = crate::kotlin_home::KotlinHome::command("kotlinc")
            .arg("-version")
            .output()
            .is_err();
        assert!(
            !missing || std::env::var_os("CI").is_none(),
            "CI must provide kotlinc"
        );
        missing
    }
}

fn kotlin_test_settings_reach_the_setup_gate_in(unit: &str) {
    if KotlinTestSettingsProject::kotlinc_missing() {
        eprintln!("skipping: kotlinc is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("test project");
    KotlinTestSettingsProject::write_project(root.path());
    let (name, _, _, refusal) = KotlinTestSettingsProject::UNITS
        .into_iter()
        .find(|(name, _, _, _)| *name == unit)
        .unwrap_or_else(|| panic!("no Kotlin settings unit is named {unit}"));
    let relative = format!("client/tests/{name}.kt");
    let outcome = super::run(root.path(), Path::new(&relative));
    match refusal {
        None => {
            let report = outcome.expect("native Kotlin test unit");
            let outcomes: Vec<(&str, bool)> = report
                .cases
                .iter()
                .map(|case| (case.name.as_str(), case.passed))
                .collect();
            assert_eq!(
                outcomes,
                [("testTheSetupIsActive", true)],
                "{name}: {report:?}"
            );
        }
        Some(code) => {
            let failure = format!(
                "{:?}",
                outcome.expect_err("an inactive setup stops the unit")
            );
            assert!(failure.contains(code), "{name}: {failure}");
        }
    }
}

one_test_per_case!(
    kotlin_test_settings_reach_the_setup_gate_as_tac_settings,
    kotlin_test_settings_reach_the_setup_gate_in,
    all KotlinTestSettingsProject::UNITS.map(|(name, _, _, _)| name),
    [
        cache_settings => "CacheSettingsTest",
        page_settings => "PageSettingsTest",
        title_settings => "TitleSettingsTest",
        cache_off => "CacheOffTest",
        no_settings => "NoSettingsTest",
    ]
);
