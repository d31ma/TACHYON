#![allow(clippy::expect_used)]
use tachyon_contracts::{HandlerRequest, HttpMethod};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
};

struct AssertedUnit;

impl AssertedUnit {
    const JAVASCRIPT_SETUP: &'static str =
        "@TestSetup()\nclass CatalogTestSetup {\n  static configure() { return {}; }\n}\n";
    const JAVASCRIPT_UNIT: &'static str = "@Test(CatalogTestSetup)\nexport default class CatalogServiceTest {\n  async testAddsTwoNumbers() {\n    Assert.equals(4, 2 + 2);\n    Assert.equals({ items: [1, 2] }, { items: [1, 2] });\n  }\n\n  async testExplainsAWrongTotal() {\n    Assert.equals(3, 1 + 1, 'the total');\n  }\n\n  async testSeesAThrownError() {\n    const error = Assert.throws(() => { throw new Error('empty name'); });\n    Assert.equals('empty name', error.message);\n    await Assert.throws(async () => Promise.reject(new Error('later')));\n  }\n\n  async testNeedsAThrownError() {\n    Assert.throws(() => 1);\n  }\n\n  @ValueSource([1, 2, -3])\n  async testKeepsPositiveInputs(value) {\n    Assert.isTrue(value > 0, 'a positive input');\n  }\n}\n";
    const PYTHON_SETUP: &'static str = "@TestSetup()\nclass CatalogTestSetup:\n    @staticmethod\n    def configure(execution):\n        return {}\n";
    const PYTHON_UNIT: &'static str = "@Test(CatalogTestSetup)\nclass CatalogServiceTest:\n    async def test_adds_two_numbers(self):\n        Assert.equals(4, 2 + 2)\n        assert [1, 2] == [1, 2]\n\n    async def test_explains_a_wrong_total(self):\n        Assert.equals(3, 1 + 1, 'the total')\n\n    async def test_sees_a_raised_error(self):\n        error = Assert.throws(lambda: int('x'))\n        Assert.is_true(isinstance(error, ValueError))\n\n    async def test_needs_a_raised_error(self):\n        Assert.throws(lambda: 1)\n\n    @ValueSource([1, 2, -3])\n    async def test_keeps_positive_inputs(self, value):\n        assert value > 0, 'a positive input'\n";
    const RUST_SETUP: &'static str = "#[TestSetup()]\nstruct CatalogTestSetup {}\nimpl CatalogTestSetup {\n    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> { Ok(BehaviorConfiguration::default()) }\n}\n";
    const RUST_UNIT: &'static str = "#[Test(CatalogTestSetup)]\nstruct CatalogServiceTest { behaviors: std::cell::RefCell<Option<BehaviorContext>> }\nimpl CatalogServiceTest {\n    fn new() -> Self { Self { behaviors: std::cell::RefCell::new(None) } }\n    fn testAddsTwoNumbers(&self) {\n        Assert::equals(4, 2 + 2);\n        assert_eq!(String::from(\"ok\"), \"ok\");\n    }\n    fn testExplainsAWrongTotal(&self) {\n        Assert::equals_with_message(3, 1 + 1, \"the total\");\n    }\n    fn testSeesAnError(&self) -> BehaviorResult<BehaviorValue> {\n        Assert::throws(|| -> BehaviorResult<()> { Err(BehaviorError { code: \"adapter\", message: \"empty name\", path: None }) });\n        Assert::throws(|| -> Result<(), String> { panic!(\"stopped\") });\n        Ok(BehaviorValue::Null)\n    }\n    fn testNeedsAnError(&self) {\n        Assert::throws(|| -> Result<i32, String> { Ok(1) });\n    }\n    #[ValueSource([1, 2, -3])]\n    fn testKeepsPositiveInputs(&self, value: i64) {\n        assert!(value > 0, \"a positive input\");\n    }\n}\n";

    fn write(root: &std::path::Path, relative: &str, source: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directories");
        std::fs::write(path, source).expect("source");
    }

    fn available(program: &str) -> bool {
        std::process::Command::new(program)
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }

    async fn server_cases(extension: &str, setup: &str, unit: &str) -> Vec<(String, bool, String)> {
        let root = tempfile::tempdir().expect("project");
        Self::write(
            root.path(),
            &format!("server/tests/CatalogTestSetup.{extension}"),
            setup,
        );
        let relative = format!("server/tests/CatalogServiceTest.{extension}");
        Self::write(root.path(), &relative, unit);
        let source = HandlerSource::discover(root.path(), &relative)
            .unwrap_or_else(|error| panic!("{extension} discovery: {error:?}"));
        let mut options = HandlerSupervisorOptions::default();
        if extension == "ts" {
            options.runtimes.javascript = "bun".into();
        }
        let supervisor = HandlerSupervisor::new(options).expect("supervisor");
        let response = supervisor
            .invoke(
                &source,
                &HandlerRequest::route("test_unit", "/", HttpMethod::Post),
                &HandlerCancellation::default(),
            )
            .await
            .unwrap_or_else(|error| panic!("{extension} invocation: {error:?}"));
        assert!(
            response.error.is_none(),
            "{extension}: {:?}",
            response.error
        );
        let body = response.body.expect("body").data;
        let report: serde_json::Value = serde_json::from_str(&body).expect("JSON report");
        report["cases"]
            .as_array()
            .expect("cases")
            .iter()
            .map(|case| {
                (
                    case["name"].as_str().unwrap_or_default().to_owned(),
                    case["passed"] == true,
                    case["error"].as_str().unwrap_or_default().to_owned(),
                )
            })
            .collect()
    }

    fn client_cases(extension: &str, setup: &str, unit: &str) -> Vec<(String, bool, String)> {
        let root = tempfile::tempdir().expect("project");
        Self::write(
            root.path(),
            &format!("client/tests/CatalogTestSetup.{extension}"),
            setup,
        );
        let relative = format!("client/tests/CatalogServiceTest.{extension}");
        Self::write(root.path(), &relative, unit);
        let report = tachyon_core::client_tests::run(root.path(), std::path::Path::new(&relative))
            .unwrap_or_else(|error| panic!("{extension}: {error:?}"));
        report
            .cases
            .into_iter()
            .map(|case| (case.name, case.passed, case.error.unwrap_or_default()))
            .collect()
    }

    fn check(extension: &str, cases: &[(String, bool, String)], expected: &[(&str, bool, &str)]) {
        let names: Vec<&str> = cases.iter().map(|(name, _, _)| name.as_str()).collect();
        let wanted: Vec<&str> = expected.iter().map(|(name, _, _)| *name).collect();
        assert_eq!(names, wanted, "{extension}: {cases:?}");
        for ((name, passed, error), (_, expected_pass, expected_error)) in
            cases.iter().zip(expected)
        {
            assert_eq!(passed, expected_pass, "{extension} {name}: {error}");
            assert!(
                error.contains(expected_error),
                "{extension} {name}: {error}"
            );
        }
    }

    fn javascript_expected() -> [(&'static str, bool, &'static str); 7] {
        [
            ("testAddsTwoNumbers", true, ""),
            (
                "testExplainsAWrongTotal",
                false,
                "the total ==> expected: <3> but was: <2>",
            ),
            ("testSeesAThrownError", true, ""),
            (
                "testNeedsAThrownError",
                false,
                "Expected an error to be thrown, but nothing was thrown.",
            ),
            ("testKeepsPositiveInputs[1]", true, ""),
            ("testKeepsPositiveInputs[2]", true, ""),
            (
                "testKeepsPositiveInputs[3]",
                false,
                "a positive input ==> expected: <true> but was: <false>",
            ),
        ]
    }

    fn rust_expected() -> [(&'static str, bool, &'static str); 7] {
        [
            ("testAddsTwoNumbers", true, ""),
            (
                "testExplainsAWrongTotal",
                false,
                "the total ==> expected: <3> but was: <2>",
            ),
            ("testSeesAnError", true, ""),
            (
                "testNeedsAnError",
                false,
                "Expected an error to be thrown, but nothing was thrown.",
            ),
            ("testKeepsPositiveInputs[1]", true, ""),
            ("testKeepsPositiveInputs[2]", true, ""),
            ("testKeepsPositiveInputs[3]", false, "a positive input"),
        ]
    }
}

#[tokio::test]
async fn javascript_and_typescript_units_assert_and_run_one_case_per_input() {
    for extension in ["js", "ts"] {
        if extension == "ts" && !AssertedUnit::available("bun") {
            eprintln!("skipping the TypeScript unit: bun is unavailable");
            continue;
        }
        let unit = AssertedUnit::JAVASCRIPT_UNIT.replace(
            "(value)",
            &format!("(value{})", if extension == "ts" { ": number" } else { "" }),
        );
        let cases =
            AssertedUnit::server_cases(extension, AssertedUnit::JAVASCRIPT_SETUP, &unit).await;
        AssertedUnit::check(extension, &cases, &AssertedUnit::javascript_expected());
    }
}

#[tokio::test]
async fn a_python_unit_asserts_and_runs_one_case_per_input() {
    if !AssertedUnit::available("python3") {
        eprintln!("skipping: python3 is unavailable");
        return;
    }
    let cases =
        AssertedUnit::server_cases("py", AssertedUnit::PYTHON_SETUP, AssertedUnit::PYTHON_UNIT)
            .await;
    AssertedUnit::check(
        "py",
        &cases,
        &[
            ("test_adds_two_numbers", true, ""),
            (
                "test_explains_a_wrong_total",
                false,
                "the total ==> expected: <3> but was: <2>",
            ),
            ("test_sees_a_raised_error", true, ""),
            (
                "test_needs_a_raised_error",
                false,
                "Expected an error to be thrown, but nothing was thrown.",
            ),
            ("test_keeps_positive_inputs[1]", true, ""),
            ("test_keeps_positive_inputs[2]", true, ""),
            ("test_keeps_positive_inputs[3]", false, "a positive input"),
        ],
    );
}

#[tokio::test]
async fn a_rust_unit_reports_a_failed_assert_as_a_failed_case() {
    if !AssertedUnit::available("rustc") {
        eprintln!("skipping: rustc is unavailable");
        return;
    }
    let cases =
        AssertedUnit::server_cases("rs", AssertedUnit::RUST_SETUP, AssertedUnit::RUST_UNIT).await;
    AssertedUnit::check("rs", &cases, &AssertedUnit::rust_expected());
}

#[test]
fn tac_units_assert_and_run_one_case_per_input() {
    let javascript = AssertedUnit::client_cases(
        "js",
        AssertedUnit::JAVASCRIPT_SETUP,
        AssertedUnit::JAVASCRIPT_UNIT,
    );
    AssertedUnit::check("tac js", &javascript, &AssertedUnit::javascript_expected());
    if !AssertedUnit::available("rustc") {
        eprintln!("skipping the Rust unit: rustc is unavailable");
        return;
    }
    let rust = AssertedUnit::client_cases("rs", AssertedUnit::RUST_SETUP, AssertedUnit::RUST_UNIT);
    AssertedUnit::check("tac rs", &rust, &AssertedUnit::rust_expected());
}
