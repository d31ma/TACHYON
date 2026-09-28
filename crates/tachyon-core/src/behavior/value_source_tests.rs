use super::lower;
use std::path::Path;

struct ValueSourceCase;

impl ValueSourceCase {
    const SETUP: &'static str =
        "@TestSetup()\nclass CatalogTestSetup {\n  static configure() { return {}; }\n}\n";
    const RUST_SETUP: &'static str = "#[TestSetup()]\nstruct CatalogTestSetup {}\nimpl CatalogTestSetup {\n    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> { Ok(BehaviorConfiguration::default()) }\n}\n";

    fn javascript(method: &str) -> Result<Option<String>, crate::Failure> {
        let source = format!(
            "{}@Test(CatalogTestSetup)\nexport default class CatalogServiceTest {{\n{method}\n}}\n",
            Self::SETUP
        );
        lower(
            Path::new("server/tests/CatalogServiceTest.js"),
            &source,
            "yon",
        )
    }

    fn rust(method: &str) -> Result<Option<String>, crate::Failure> {
        let source = format!(
            "{}#[Test(CatalogTestSetup)]\nstruct CatalogServiceTest {{ behaviors: std::cell::RefCell<Option<BehaviorContext>> }}\nimpl CatalogServiceTest {{\n    fn new() -> Self {{ Self {{ behaviors: std::cell::RefCell::new(None) }} }}\n{method}\n}}\n",
            Self::RUST_SETUP
        );
        lower(
            Path::new("server/tests/CatalogServiceTest.rs"),
            &source,
            "yon",
        )
    }

    fn refusal(result: Result<Option<String>, crate::Failure>) -> String {
        let Err(failure) = result else {
            panic!("the source was accepted");
        };
        failure.diagnostics()[0].message.clone()
    }
}

#[test]
fn every_input_becomes_one_named_case() {
    let lowered = ValueSourceCase::javascript(
        "  @ValueSource([2, \"two\"])\n  async testKeepsAnInput(value) {\n    Assert.isTrue(value !== undefined)\n  }\n  async testRunsOnce() {\n    Assert.isTrue(true)\n  }",
    )
    .expect("lowered")
    .expect("a test unit");
    assert!(
        lowered.contains(r#"[["testKeepsAnInput[1]","testKeepsAnInput",[2]],["testKeepsAnInput[2]","testKeepsAnInput",["two"]],["testRunsOnce","testRunsOnce",[]]]"#),
        "{lowered}"
    );
    assert!(lowered.contains("class Assert"), "{lowered}");
}

#[test]
fn value_source_goes_on_a_test_case_with_a_list_of_literals() {
    for (method, expected) in [
        (
            "  @ValueSource([1])\n  async keepsAnInput(value) {\n    Assert.isTrue(value > 0)\n  }",
            "a method whose name begins with `test`",
        ),
        (
            "  @ValueSource(1)\n  async testKeepsAnInput(value) {\n    Assert.isTrue(value > 0)\n  }",
            "one list of 1 to 256 literal inputs",
        ),
        (
            "  @ValueSource([])\n  async testKeepsAnInput(value) {\n    Assert.isTrue(value > 0)\n  }",
            "one list of 1 to 256 literal inputs",
        ),
        (
            "  @ValueSource([1])\n  @ValueSource([2])\n  async testKeepsAnInput(value) {\n    Assert.isTrue(value > 0)\n  }",
            "at most one ValueSource",
        ),
    ] {
        let message = ValueSourceCase::refusal(ValueSourceCase::javascript(method));
        assert!(message.contains(expected), "{message}");
    }
}

#[test]
fn a_rust_value_source_lists_numbers_strings_or_booleans() {
    let accepted = ValueSourceCase::rust(
        "    #[ValueSource([1, \"two\", true])]\n    fn test_keeps_an_input(&self, value: impl std::fmt::Debug) {\n        Assert::is_true(!format!(\"{value:?}\").is_empty());\n    }",
    )
    .expect("lowered")
    .expect("a test unit");
    for call in [
        "\"test_keeps_an_input[1]\" => CaseOutcome::failure(self.test_keeps_an_input(1))",
        "\"test_keeps_an_input[2]\" => CaseOutcome::failure(self.test_keeps_an_input(\"two\"))",
        "\"test_keeps_an_input[3]\" => CaseOutcome::failure(self.test_keeps_an_input(true))",
    ] {
        assert!(accepted.contains(call), "{call} missing from {accepted}");
    }
    let message = ValueSourceCase::refusal(ValueSourceCase::rust(
        "    #[ValueSource([[1, 2]])]\n    fn test_keeps_an_input(&self, value: Vec<i64>) {\n        Assert::is_true(!value.is_empty());\n    }",
    ));
    assert!(
        message.contains("numbers, strings or booleans"),
        "{message}"
    );
}
