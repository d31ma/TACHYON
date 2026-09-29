use super::lower;
use std::path::Path;

struct ValueSourceCase;

impl ValueSourceCase {
    const SETUP: &'static str =
        "@TestSetup()\nclass CatalogTestSetup {\n  static configure() { return {}; }\n}\n";
    const RUST_SETUP: &'static str = "#[TestSetup()]\nstruct CatalogTestSetup {}\nimpl CatalogTestSetup {\n    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> { Ok(BehaviorConfiguration::default()) }\n}\n";

    const JAVA_SETUP: &'static str = "@TestSetup\nclass CatalogTestSetup {\n    static Behaviors.Configuration configure(Behaviors.Execution execution) {\n        return null;\n    }\n}\n";
    const KOTLIN_SETUP: &'static str = "@TestSetup\nclass CatalogTestSetup {\n    companion object {\n        fun configure(execution: Behaviors.Execution): Behaviors.Configuration = Behaviors.Configuration()\n    }\n}\n";

    const CSHARP_SETUP: &'static str = "[TestSetup]\npublic sealed class CatalogTestSetup {\n    public static BehaviorAdapters configure(BehaviorExecution execution) { return new BehaviorAdapters(); }\n}\n";

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

    fn jvm(extension: &str, method: &str) -> Result<Option<String>, crate::Failure> {
        let (setup, class) = if extension == "kt" {
            (Self::KOTLIN_SETUP, "::class")
        } else {
            (Self::JAVA_SETUP, ".class")
        };
        let source = format!(
            "{setup}@Test(CatalogTestSetup{class})\nclass CatalogServiceTest {{\n{method}\n}}\n"
        );
        lower(
            Path::new(&format!("server/tests/CatalogServiceTest.{extension}")),
            &source,
            "yon",
        )
    }

    fn csharp(method: &str) -> Result<Option<String>, crate::Failure> {
        let source = format!(
            "{}[Test(typeof(CatalogTestSetup), {{\"partition\":\"tests\"}})]\nsealed class CatalogServiceTest {{\n{method}\n}}\n",
            Self::CSHARP_SETUP
        );
        lower(
            Path::new("server/tests/CatalogServiceTest.cs"),
            &source,
            "yon",
        )
    }

    fn native(
        relative: &str,
        source: &str,
        environment: &str,
    ) -> Result<Option<String>, crate::Failure> {
        lower(Path::new(relative), source, environment)
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
        "\"test_keeps_an_input[1]\" => CaseOutcome::from(self.test_keeps_an_input(1)).failure()",
        "\"test_keeps_an_input[2]\" => CaseOutcome::from(self.test_keeps_an_input(\"two\")).failure()",
        "\"test_keeps_an_input[3]\" => CaseOutcome::from(self.test_keeps_an_input(true)).failure()",
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

#[test]
fn a_jvm_value_source_calls_the_case_once_for_each_literal() {
    let cases = [
        (
            "java",
            "    @ValueSource([1, \"a$b\", true, 3000000000])\n    void testKeepsAnInput(Object value) {\n        Assert.isTrue(value != null);\n    }",
        ),
        (
            "groovy",
            "    @ValueSource([1, \"a$b\", true, 3000000000])\n    void testKeepsAnInput(Object value) {\n        Assert.isTrue(value != null)\n    }",
        ),
        (
            "kt",
            "    @ValueSource([1, \"a$b\", true, 3000000000])\n    fun testKeepsAnInput(value: Any) {\n        Assert.isTrue(value != null)\n    }",
        ),
    ];
    for (extension, method) in cases {
        let lowered = ValueSourceCase::jvm(extension, method)
            .expect("lowered")
            .expect("a test unit");
        for call in [
            "\"testKeepsAnInput[1]\"",
            "testKeepsAnInput(1)",
            "testKeepsAnInput(\"a\\u0024b\")",
            "testKeepsAnInput(true)",
            "testKeepsAnInput(3000000000L)",
            if extension == "kt" {
                "object Assert"
            } else {
                "final class Assert"
            },
        ] {
            assert!(
                lowered.contains(call),
                "{extension}: {call} missing from {lowered}"
            );
        }
    }
}

#[test]
fn a_jvm_value_source_lists_numbers_strings_or_booleans() {
    for (extension, method) in [
        (
            "java",
            "    @ValueSource([[1, 2]])\n    void testKeepsAnInput(Object value) {\n        Assert.isTrue(value != null);\n    }",
        ),
        (
            "kt",
            "    @ValueSource([[1, 2]])\n    fun testKeepsAnInput(value: Any) {\n        Assert.isTrue(value != null)\n    }",
        ),
    ] {
        let message = ValueSourceCase::refusal(ValueSourceCase::jvm(extension, method));
        assert!(
            message.contains("numbers, strings or booleans"),
            "{extension}: {message}"
        );
    }
}

#[test]
fn a_csharp_value_source_calls_the_case_once_for_each_literal() {
    let method = "    [ValueSource([1, \"a\\\"b\", true, 3000000000])]\n    void testKeepsAnInput(object value) {\n        Assert.isTrue(value != null);\n    }";
    let lowered = ValueSourceCase::csharp(method)
        .expect("lowered")
        .expect("a test unit");
    for call in [
        "case \"testKeepsAnInput[1]\": testKeepsAnInput(1); return;",
        "testKeepsAnInput(\"a\\\"b\")",
        "testKeepsAnInput(true)",
        "testKeepsAnInput(3000000000)",
        "\"testKeepsAnInput[4]\"",
        "static class Assert",
    ] {
        assert!(lowered.contains(call), "{call} missing from {lowered}");
    }
}

#[test]
fn a_csharp_value_source_lists_numbers_strings_or_booleans() {
    let method = "    [ValueSource([[1, 2]])]\n    void testKeepsAnInput(object value) {\n        Assert.isTrue(value != null);\n    }";
    let message = ValueSourceCase::refusal(ValueSourceCase::csharp(method));
    assert!(
        message.contains("numbers, strings or booleans"),
        "{message}"
    );
}

#[test]
fn a_csharp_module_without_a_test_class_has_no_assert() {
    let source = "public sealed class CatalogService {\n    [Needs]\n    public BehaviorContext behaviors;\n    [Retry({\"maxRetries\":1,\"delayMs\":0,\"multiplier\":1,\"maxDelayMs\":10,\"jitterMs\":0})]\n    public async System.Threading.Tasks.Task<BehaviorJSON> load() {\n        await System.Threading.Tasks.Task.CompletedTask;\n        return BehaviorJSON.From(1);\n    }\n}\n";
    let lowered = lower(
        Path::new("server/services/CatalogService.cs"),
        source,
        "yon",
    )
    .expect("lowered")
    .expect("a service with a behavior");
    assert!(!lowered.contains("class Assert"), "{lowered}");
}

#[test]
fn a_go_value_source_calls_the_case_once_for_each_literal() {
    let source = "// @Test(CatalogTestSetup)\ntype CatalogServiceTest struct {\n\tbehaviors *BehaviorContext\n}\n\n// @ValueSource([1, \"a\\\"b\", true])\nfunc (test *CatalogServiceTest) TestKeepsAnInput(value any) {\n\tAssert.IsTrue(value != nil)\n}\n\nfunc (test *CatalogServiceTest) TestStillReturnsAnError() error {\n\treturn nil\n}\n";
    let lowered = ValueSourceCase::native("server/tests/CatalogServiceTest.go", source, "yon")
        .expect("lowered")
        .expect("a test unit");
    for call in [
        "case \"TestKeepsAnInput[1]\":\n\t\t__tachyon_receiver.TestKeepsAnInput(1)\n\t\treturn nil",
        "__tachyon_receiver.TestKeepsAnInput(\"a\\\"b\")",
        "__tachyon_receiver.TestKeepsAnInput(true)",
        "return __tachyon_receiver.TestStillReturnsAnError()",
        "var Assert BehaviorAssert",
    ] {
        assert!(lowered.contains(call), "{call} missing from {lowered}");
    }
}

#[test]
fn a_go_value_source_lists_numbers_strings_or_booleans() {
    let source = "// @Test(CatalogTestSetup)\ntype CatalogServiceTest struct {\n\tbehaviors *BehaviorContext\n}\n\n// @ValueSource([[1, 2]])\nfunc (test *CatalogServiceTest) TestKeepsAnInput(value []int) {\n\tAssert.IsTrue(len(value) > 0)\n}\n";
    let message = ValueSourceCase::refusal(ValueSourceCase::native(
        "server/tests/CatalogServiceTest.go",
        source,
        "yon",
    ));
    assert!(
        message.contains("numbers, strings or booleans"),
        "{message}"
    );
}

#[test]
fn a_dart_value_source_keeps_a_dollar_sign_literal() {
    let source = "@Test(CatalogTestSetup)\nclass CatalogServiceTest {\n  @ValueSource([2, \"a$b\"])\n  Future<void> testKeepsAnInput(Object value) async {\n    Assert.isTrue(value != null);\n  }\n}\n";
    let lowered = ValueSourceCase::native("server/tests/CatalogServiceTest.dart", source, "yon")
        .expect("lowered")
        .expect("a test unit");
    for call in [
        "case 'testKeepsAnInput[1]':\n        await testKeepsAnInput(2);",
        "await testKeepsAnInput(\"a\\$b\");",
        "class Assert",
    ] {
        assert!(lowered.contains(call), "{call} missing from {lowered}");
    }
}

#[test]
fn a_swift_value_source_calls_the_case_with_swift_literals() {
    let source = "@Test(CatalogTestSetup)\nfinal class CatalogServiceTest {\n  @ValueSource([\"a\\\"b\", 2.5])\n  func testKeepsAnInput(_ value: Any) async throws {\n    try Assert.isTrue(true)\n  }\n}\n";
    let lowered = ValueSourceCase::native("client/tests/CatalogServiceTest.swift", source, "tac")
        .expect("lowered")
        .expect("a test unit");
    for call in [
        "case \"testKeepsAnInput[1]\": try await testKeepsAnInput(\"a\\\"b\")",
        "try await testKeepsAnInput(2.5)",
        "enum Assert",
    ] {
        assert!(lowered.contains(call), "{call} missing from {lowered}");
    }
}

#[test]
fn ruby_and_php_value_sources_pass_any_json_input() {
    let ruby = "#[Test(CatalogTestSetup)]\nclass CatalogServiceTest\n  #[ValueSource([[1, 2], {\"a\": 1}])]\n  def test_keeps_an_input(value)\n    Assert.is_true(!value.nil?)\n  end\nend\n";
    let lowered = ValueSourceCase::native("server/tests/CatalogServiceTest.rb", ruby, "yon")
        .expect("lowered")
        .expect("a test unit");
    assert!(lowered.contains("test_keeps_an_input[2]"), "{lowered}");
    assert!(lowered.contains("class Assert"), "{lowered}");
    let php = "<?php\n#[Test(CatalogTestSetup::class)]\nclass CatalogServiceTest {\n    #[ValueSource([[1, 2], {\"a\": 1}])]\n    public function testKeepsAnInput($value): void {\n        Assert::isTrue($value !== null);\n    }\n}\n";
    let lowered = ValueSourceCase::native("server/tests/CatalogServiceTest.php", php, "yon")
        .expect("lowered")
        .expect("a test unit");
    assert!(lowered.contains("testKeepsAnInput[2]"), "{lowered}");
    assert!(lowered.contains("final class Assert"), "{lowered}");
}
