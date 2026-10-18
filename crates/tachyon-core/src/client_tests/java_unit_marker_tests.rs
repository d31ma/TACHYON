#![allow(clippy::expect_used)]

use std::path::Path;

struct JavaUnitMarkerProject;

impl JavaUnitMarkerProject {
    const SETUP_PATH: &'static str = "client/tests/CatalogTestSetup.java";
    const SETUP: &'static str = r#"@TestSetup
@Settings({"prefix":"TAC_CATALOG_"})
@WhenProfile("test")
@WhenSetting({"name":"TAC_CATALOG_CACHE_ENABLED","havingValue":"true","matchIfMissing":true})
class CatalogTestSetup {
    @Min(1)
    @Max(100)
    int pageSize = 20;
    boolean cacheEnabled = false;
    String title = "Catalog";
    static Behaviors.Configuration configure(Behaviors.Execution execution) {
        int[] calls = {0};
        Behaviors.HttpAdapter counter = (request, current) -> ++calls[0];
        return new Behaviors.Configuration(java.util.Map.of(), java.util.Map.of("counter", counter), java.util.Map.of(), (call, current) -> true);
    }
}
"#;
    const SERVICE_PATH: &'static str = "client/services/PriceService.java";
    const SERVICE: &'static str = "@Service\nclass PriceService {\n    long withTax(long cents) { return cents + cents / 5; }\n}\n";
    const UNIT_FOLDER: &'static str = "client/tests";

    fn run(name: &str, unit: &str) -> super::TestReport {
        Self::attempt(name, unit).expect("native Java test unit")
    }

    fn refusal(name: &str, unit: &str) -> String {
        let failure = Self::attempt(name, unit).expect_err("an inactive setup stops the unit");
        format!("{failure:?}")
    }

    fn attempt(name: &str, unit: &str) -> Result<super::TestReport, crate::Failure> {
        let root = tempfile::tempdir().expect("test project");
        let relative = format!("{}/{name}.java", Self::UNIT_FOLDER);
        for (path, source) in [
            (Self::SETUP_PATH, Self::SETUP),
            (Self::SERVICE_PATH, Self::SERVICE),
            (relative.as_str(), unit),
        ] {
            let path = root.path().join(path);
            std::fs::create_dir_all(path.parent().expect("fixture parent"))
                .expect("fixture directory");
            std::fs::write(path, source).expect("fixture source");
        }
        super::run(root.path(), Path::new(&relative))
    }

    fn outcomes(report: &super::TestReport) -> Vec<(&str, bool)> {
        report
            .cases
            .iter()
            .map(|case| (case.name.as_str(), case.passed))
            .collect()
    }

    fn javac_missing() -> bool {
        let missing = std::process::Command::new("javac")
            .arg("-version")
            .output()
            .is_err();
        assert!(
            !missing || std::env::var_os("CI").is_none(),
            "CI must provide javac"
        );
        missing
    }
}

#[test]
fn java_value_source_runs_a_case_once_for_each_input() {
    if JavaUnitMarkerProject::javac_missing() {
        return;
    }
    let report = JavaUnitMarkerProject::run(
        "PriceServiceTest",
        r#"@Test(CatalogTestSetup.class)
@UseProfiles("test")
class PriceServiceTest {
    @ValueSource([0, 5, 1000])
    void testNeverLowersAPrice(long cents) {
        Assert.isTrue(new PriceService().withTax(cents) >= cents, "the tax never lowers a price");
    }
    @ValueSource(["anvil", "rope"])
    void testOnlyAcceptsAnvils(String name) {
        Assert.equals("anvil", name);
    }
}
"#,
    );
    assert_eq!(
        JavaUnitMarkerProject::outcomes(&report),
        [
            ("testNeverLowersAPrice[1]", true),
            ("testNeverLowersAPrice[2]", true),
            ("testNeverLowersAPrice[3]", true),
            ("testOnlyAcceptsAnvils[1]", true),
            ("testOnlyAcceptsAnvils[2]", false),
        ],
        "{report:?}"
    );
}

#[test]
fn java_use_profiles_selects_a_setup_that_only_its_profile_activates() {
    if JavaUnitMarkerProject::javac_missing() {
        return;
    }
    let report = JavaUnitMarkerProject::run(
        "ProfileTest",
        r#"@Test(CatalogTestSetup.class, {"partition":"tests"})
@UseProfiles("test")
class ProfileTest {
    void testTheContextCarriesTheProfile() {
        Assert.equals(java.util.List.of("test"), behaviors.options.profiles);
        Assert.equals("tests", behaviors.options.partition);
    }
}
"#,
    );
    assert_eq!(
        JavaUnitMarkerProject::outcomes(&report),
        [("testTheContextCarriesTheProfile", true)],
        "{report:?}"
    );
    let refusal = JavaUnitMarkerProject::refusal(
        "NoProfileTest",
        "@Test(CatalogTestSetup.class)\nclass NoProfileTest {\n    void testNeedsTheProfile() {}\n}\n",
    );
    assert!(refusal.contains("inactive-configuration"), "{refusal}");
}

#[test]
fn java_test_settings_reach_the_context_as_tac_settings() {
    if JavaUnitMarkerProject::javac_missing() {
        return;
    }
    let report = JavaUnitMarkerProject::run(
        "SettingsTest",
        r#"@Test(CatalogTestSetup.class)
@UseProfiles("test")
@TestSettings({"tac.catalog.page.size":5,"tac.catalog.cache.enabled":true,"tac.catalog.title":"Test catalog"})
class SettingsTest {
    void testTheSetupBindsTheSettings() {
        java.util.Map<String, Object> catalog = Behaviors.properties("CatalogTestSetup", behaviors.options.properties);
        Assert.equals(5L, catalog.get("pageSize"));
        Assert.equals(true, catalog.get("cacheEnabled"));
        Assert.equals("Test catalog", catalog.get("title"));
    }
}
"#,
    );
    assert_eq!(
        JavaUnitMarkerProject::outcomes(&report),
        [("testTheSetupBindsTheSettings", true)],
        "{report:?}"
    );
    let refusal = JavaUnitMarkerProject::refusal(
        "DisabledTest",
        r#"@Test(CatalogTestSetup.class)
@UseProfiles("test")
@TestSettings({"tac.catalog.cache.enabled":false})
class DisabledTest {
    void testTheSettingTurnsTheSetupOff() {}
}
"#,
    );
    assert!(refusal.contains("inactive-configuration"), "{refusal}");
}

#[test]
fn java_fresh_start_gives_each_case_a_new_setup() {
    if JavaUnitMarkerProject::javac_missing() {
        return;
    }
    let report = JavaUnitMarkerProject::run(
        "FreshTest",
        r#"@Test(CatalogTestSetup.class)
@UseProfiles("test")
@FreshStart
class FreshTest {
    Object count() { return behaviors.configuration.http.get("counter").exchange(java.util.Map.of(), null); }
    void testFirstCaseCountsFromOne() {
        Assert.equals(1, count());
        Assert.equals(2, count());
    }
    void testSecondCaseCountsFromOneAgain() {
        Assert.equals(1, count());
    }
}
"#,
    );
    assert_eq!(
        JavaUnitMarkerProject::outcomes(&report),
        [
            ("testFirstCaseCountsFromOne", true),
            ("testSecondCaseCountsFromOneAgain", true),
        ],
        "{report:?}"
    );
}
