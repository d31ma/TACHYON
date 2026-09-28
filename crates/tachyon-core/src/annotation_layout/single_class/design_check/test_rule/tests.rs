use super::super::DesignCheck;
use super::super::DesignSwitches;
use super::super::project_words::ProjectWords;
use cap_std::ambient_authority;
use cap_std::fs::Dir;
use std::path::Path;

struct TestedProject;

impl TestedProject {
    const CODE: u16 = 1339;

    fn messages(files: &[(&str, &str)]) -> Vec<String> {
        let switches = DesignSwitches::every_realm(ProjectWords::default());
        Self::found(files, &switches)
    }

    fn found(files: &[(&str, &str)], switches: &DesignSwitches) -> Vec<String> {
        let pairs = files
            .iter()
            .map(|(path, text)| (Path::new(*path), text.as_bytes()));
        DesignCheck::diagnostics(pairs, switches)
            .into_iter()
            .filter(|problem| problem.code.number() == Self::CODE)
            .map(|problem| problem.message)
            .collect()
    }

    fn with_settings(files: &[(&str, &str)], settings: &[(&str, &str)]) -> Vec<String> {
        let folder = tempfile::tempdir().expect("temporary project");
        for (path, text) in settings {
            let file = folder.path().join(path);
            std::fs::create_dir_all(file.parent().expect("settings folder")).expect("folder");
            std::fs::write(file, text).expect("settings file");
        }
        let project = Dir::open_ambient_dir(folder.path(), ambient_authority()).expect("project");
        Self::found(files, &DesignSwitches::read(&project))
    }
}

const JAVASCRIPT_UNIT: &str = "@Test(CatalogTestSetup)\nexport default class CatalogServiceTest {\n  twice(value) {\n    if (value < 0) return 0\n    return value * 2\n  }\n\n  async testDoublesAValue() {\n    Assert.equals(8, this.twice(4))\n  }\n\n  async testKeepsBothChecks() {\n    Assert.isTrue(this.twice(1) === 2 && this.twice(2) === 4, 'both double')\n  }\n\n  async testChecksWithAnIf() {\n    if (this.twice(4) !== 8) throw Error('expected 8')\n  }\n\n  async testOnlyLogs() {\n    console.assert(this.twice(4) === 8)\n  }\n}\n";

#[test]
fn a_javascript_case_must_assert_and_hold_no_logic() {
    assert_eq!(
        TestedProject::messages(&[("server/tests/CatalogServiceTest.js", JAVASCRIPT_UNIT)]),
        [
            "The test case `testChecksWithAnIf` in CatalogServiceTest holds logic: `if`.",
            "The test case `testChecksWithAnIf` in CatalogServiceTest has no assertion.",
            "The test case `testOnlyLogs` in CatalogServiceTest has no assertion.",
        ]
    );
}

#[test]
fn a_typescript_choice_and_loop_are_logic() {
    let unit = "@Test(CartTestSetup)\nexport default class CartServiceTest {\n  async testPicksALabel(): Promise<void> {\n    const label = 2 > 1 ? 'many' : 'one'\n    Assert.equals('many', label)\n  }\n\n  async testAddsEveryItem(): Promise<void> {\n    for (const item of [1, 2]) {\n      Assert.isTrue(item > 0)\n    }\n  }\n\n  async testReadsAnOptionalName(): Promise<void> {\n    const cart: { name?: string } = {}\n    Assert.equals(undefined, cart?.name ?? undefined)\n  }\n}\n";
    assert_eq!(
        TestedProject::messages(&[("client/tests/CartServiceTest.ts", unit)]),
        [
            "The test case `testPicksALabel` in CartServiceTest holds logic: `? :`.",
            "The test case `testAddsEveryItem` in CartServiceTest holds logic: `for`.",
        ]
    );
}

#[test]
fn a_python_case_may_use_the_language_assert() {
    let unit = "@Test(CatalogTestSetup)\nclass CatalogServiceTest:\n    async def test_doubles_a_value(self):\n        assert 4 * 2 == 8\n\n    async def test_checks_every_value(self):\n        for value in [1, 2]:\n            Assert.is_true(value > 0)\n\n    async def test_picks_a_label(self):\n        Assert.equals('many', 'many' if 2 > 1 else 'one')\n\n    async def test_raises(self):\n        raise Exception('no check')\n";
    assert_eq!(
        TestedProject::messages(&[("server/tests/CatalogServiceTest.py", unit)]),
        [
            "The test case `test_checks_every_value` in CatalogServiceTest holds logic: `for`.",
            "The test case `test_picks_a_label` in CatalogServiceTest holds logic: `if`.",
            "The test case `test_raises` in CatalogServiceTest has no assertion.",
        ]
    );
}

#[test]
fn a_rust_case_may_use_the_assert_macros() {
    let unit = "#[Test(StatusTestSetup)]\nstruct StatusServiceTest { behaviors: std::cell::RefCell<Option<BehaviorContext>> }\nimpl StatusServiceTest {\n    fn new() -> Self { Self { behaviors: std::cell::RefCell::new(None) } }\n    fn testSaysOk(&self) -> BehaviorResult<BehaviorValue> {\n        assert_eq!(\"ok\", \"ok\");\n        Ok(BehaviorValue::Null)\n    }\n    fn testUsesAssert(&self) -> BehaviorResult<BehaviorValue> {\n        Assert::is_true(1 < 2);\n        Ok(BehaviorValue::Null)\n    }\n    fn testMatches(&self) -> BehaviorResult<BehaviorValue> {\n        match 1 { 1 => Ok(BehaviorValue::Null), _ => Err(BehaviorError { code: \"adapter\", message: \"not one\", path: None }) }\n    }\n}\n";
    assert_eq!(
        TestedProject::messages(&[("server/tests/StatusServiceTest.rs", unit)]),
        [
            "The test case `testMatches` in StatusServiceTest holds logic: `match`.",
            "The test case `testMatches` in StatusServiceTest has no assertion.",
        ]
    );
}

#[test]
fn only_test_units_are_checked() {
    let service = "@Service\nexport default class CatalogService {\n  testValue() {\n    if (1 > 0) return 1\n    return 0\n  }\n}\n";
    let setup = "@TestSetup\nexport default class CatalogTestSetup {\n  static configure() {\n    return {}\n  }\n}\n";
    assert!(
        TestedProject::messages(&[
            ("server/services/CatalogService.js", service),
            ("server/tests/CatalogTestSetup.js", setup),
        ])
        .is_empty()
    );
}

#[test]
fn the_rule_is_off_until_a_realm_turns_it_on() {
    let files = [("server/tests/CatalogServiceTest.js", JAVASCRIPT_UNIT)];
    assert!(TestedProject::with_settings(&files, &[]).is_empty());
    let turned_on = [("server/application.json", "{\"tests\":{\"check\":true}}")];
    assert_eq!(TestedProject::with_settings(&files, &turned_on).len(), 3);
    let client_only = [(
        "client/manifest.json",
        "{\"tac\":{\"tests\":{\"check\":true}}}",
    )];
    assert!(TestedProject::with_settings(&files, &client_only).is_empty());
}
