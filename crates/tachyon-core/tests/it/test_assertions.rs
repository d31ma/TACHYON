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
    const JAVA_SETUP: &'static str = "@TestSetup\nclass CatalogTestSetup {\n    static Behaviors.Configuration configure(Behaviors.Execution execution) {\n        return new Behaviors.Configuration(java.util.Map.of(), java.util.Map.of(), java.util.Map.of(), null, null);\n    }\n}\n";
    const JAVA_UNIT: &'static str = "@Test(CatalogTestSetup.class)\nclass CatalogServiceTest {\n    void testAddsTwoNumbers() {\n        Assert.equals(4, 2 + 2);\n        Assert.equals(4L, 2 + 2);\n        assert 2 + 2 == 4;\n    }\n    void testExplainsAWrongTotal() {\n        Assert.equals(3, 1 + 1, \"the total\");\n    }\n    void testSeesAThrownError() {\n        Throwable error = Assert.throwsError(() -> Integer.parseInt(\"x\"));\n        Assert.isTrue(error instanceof NumberFormatException);\n    }\n    void testNeedsAThrownError() {\n        Assert.throwsError(() -> Integer.parseInt(\"1\"));\n    }\n    @ValueSource([1, 2, -3])\n    void testKeepsPositiveInputs(int value) {\n        assert value > 0 : \"a positive input\";\n    }\n}\n";
    const GROOVY_SETUP: &'static str = "@TestSetup\nclass CatalogTestSetup {\n    static Behaviors.Configuration configure(Behaviors.Execution execution) {\n        return new Behaviors.Configuration([:], [:], [:], null, null)\n    }\n}\n";
    const GROOVY_UNIT: &'static str = "@Test(CatalogTestSetup.class)\nclass CatalogServiceTest {\n    void testAddsTwoNumbers() {\n        Assert.equals(4, 2 + 2)\n        assert 2 + 2 == 4\n    }\n    void testExplainsAWrongTotal() {\n        Assert.equals(3, 1 + 1, 'the total')\n    }\n    void testSeesAThrownError() {\n        Throwable error = Assert.throwsError({ Integer.parseInt('x') })\n        Assert.isTrue(error instanceof NumberFormatException)\n    }\n    void testNeedsAThrownError() {\n        Assert.throwsError({ Integer.parseInt('1') })\n    }\n    @ValueSource([1, 2, -3])\n    void testKeepsPositiveInputs(int value) {\n        assert value > 0 : 'a positive input'\n    }\n}\n";
    const KOTLIN_SETUP: &'static str = "@TestSetup\nclass CatalogTestSetup {\n    companion object {\n        fun configure(execution: Behaviors.Execution): Behaviors.Configuration = Behaviors.Configuration()\n    }\n}\n";
    const KOTLIN_UNIT: &'static str = "@Test(CatalogTestSetup::class)\nclass CatalogServiceTest {\n    fun testAddsTwoNumbers() {\n        Assert.equals(4, 2 + 2)\n        Assert.equals(4L, 2 + 2)\n        check(2 + 2 == 4)\n    }\n    fun testExplainsAWrongTotal() {\n        Assert.equals(3, 1 + 1, \"the total\")\n    }\n    fun testSeesAThrownError() {\n        val error = Assert.throws { \"x\".toInt() }\n        Assert.isTrue(error is NumberFormatException)\n    }\n    fun testNeedsAThrownError() {\n        Assert.throws { \"1\".toInt() }\n    }\n    @ValueSource([1, 2, -3])\n    fun testKeepsPositiveInputs(value: Int) {\n        require(value > 0) { \"a positive input\" }\n    }\n}\n";
    const CSHARP_SETUP: &'static str = "[TestSetup]\npublic sealed class CatalogTestSetup {\n    public static BehaviorAdapters configure(BehaviorExecution execution) { return new BehaviorAdapters(); }\n}\n";
    const CSHARP_UNIT: &'static str = "[Test(typeof(CatalogTestSetup), {\"partition\":\"tests\"})]\nsealed class CatalogServiceTest {\n    void testAddsTwoNumbers() {\n        Assert.equals(4, 2 + 2);\n        Assert.equals(4L, 2 + 2);\n        Assert.equals(new List<int> { 1, 2 }, new[] { 1, 2 });\n    }\n    void testExplainsAWrongTotal() {\n        Assert.equals(3, 1 + 1, \"the total\");\n    }\n    void testSeesAThrownError() {\n        var error = Assert.throws(() => int.Parse(\"x\"));\n        Assert.isTrue(error is FormatException);\n        Assert.throws(async () => { await Task.Yield(); throw new InvalidOperationException(\"later\"); });\n    }\n    void testNeedsAThrownError() {\n        Assert.throws(() => int.Parse(\"1\"));\n    }\n    [ValueSource([1, 2, -3])]\n    void testKeepsPositiveInputs(int value) {\n        Assert.isTrue(value > 0, \"a positive input\");\n    }\n}\n";
    const FSHARP_SETUP: &'static str = "[<TestSetup>]\ntype CatalogTestSetup() =\n    static member configure(execution: BehaviorExecution) : BehaviorAdapters =\n        BehaviorAdapters()\n";
    const FSHARP_UNIT: &'static str = "[<Test(typeof<CatalogTestSetup>)>]\ntype CatalogServiceTest() =\n    member this.testAddsTwoNumbers() : unit =\n        Assert.equals(4, 2 + 2)\n        Assert.equals(4L, 2 + 2)\n        Assert.equals([1; 2], [| 1; 2 |])\n\n    member this.testExplainsAWrongTotal() : unit =\n        Assert.equals(3, 1 + 1, \"the total\")\n\n    member this.testSeesAThrownError() : unit =\n        let error = Assert.throws(fun () -> int \"x\")\n        Assert.isTrue(error :? System.FormatException)\n        Assert.throws(fun () -> System.Threading.Tasks.Task.FromException(System.InvalidOperationException(\"later\"))) |> ignore\n\n    member this.testNeedsAThrownError() : unit =\n        Assert.throws(fun () -> int \"1\") |> ignore\n\n    [<ValueSource([1, 2, -3])>]\n    member this.testKeepsPositiveInputs(value: int) : unit =\n        Assert.isTrue(value > 0, \"a positive input\")\n";

    const GO_SETUP: &'static str = "// @TestSetup\ntype CatalogTestSetup struct{}\n\nfunc (CatalogTestSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {\n\treturn &BehaviorConfiguration{}, nil\n}\n";
    const GO_UNIT: &'static str = "import \"errors\"\n\n// @Test(CatalogTestSetup)\ntype CatalogServiceTest struct {\n\tbehaviors *BehaviorContext\n}\n\nfunc (test *CatalogServiceTest) TestAddsTwoNumbers() {\n\tAssert.Equals(4, 2+2)\n\tAssert.Equals(int64(4), 2+2)\n\tAssert.Equals([]int{1, 2}, []int{1, 2})\n}\n\nfunc (test *CatalogServiceTest) TestExplainsAWrongTotal() {\n\tAssert.Equals(3, 1+1, \"the total\")\n}\n\nfunc (test *CatalogServiceTest) TestSeesAnError() error {\n\terr := Assert.Throws(func() error { return errors.New(\"empty name\") })\n\tAssert.Equals(\"empty name\", err.Error())\n\tAssert.Throws(func() error { panic(\"stopped\") })\n\treturn nil\n}\n\nfunc (test *CatalogServiceTest) TestNeedsAnError() {\n\tAssert.Throws(func() error { return nil })\n}\n\n// @ValueSource([1, 2, -3])\nfunc (test *CatalogServiceTest) TestKeepsPositiveInputs(value int) {\n\tAssert.IsTrue(value > 0, \"a positive input\")\n}\n";
    const RUBY_SETUP: &'static str = "#[TestSetup]\nclass CatalogTestSetup\n  def self.configure(execution)\n    {}\n  end\nend\n";
    const RUBY_UNIT: &'static str = "#[Test(CatalogTestSetup)]\nclass CatalogServiceTest\n  def test_adds_two_numbers\n    Assert.equals(4, 2 + 2)\n    Assert.equals({ \"items\" => [1, 2] }, { \"items\" => [1, 2] })\n  end\n\n  def test_explains_a_wrong_total\n    Assert.equals(3, 1 + 1, \"the total\")\n  end\n\n  def test_sees_a_raised_error\n    error = Assert.throws { Integer(\"x\") }\n    Assert.is_true(error.is_a?(ArgumentError))\n  end\n\n  def test_needs_a_raised_error\n    Assert.throws { 1 }\n  end\n\n  #[ValueSource([1, 2, -3])]\n  def test_keeps_positive_inputs(value)\n    Assert.is_true(value > 0, \"a positive input\")\n  end\nend\n";
    const PHP_SETUP: &'static str = "<?php\n#[TestSetup]\nclass CatalogTestSetup {\n    public static function configure($execution) { return []; }\n}\n";
    const PHP_UNIT: &'static str = "<?php\n#[Test(CatalogTestSetup::class)]\nclass CatalogServiceTest {\n    public function testAddsTwoNumbers(): void {\n        Assert::equals(4, 2 + 2);\n        Assert::equals(['items' => [1, 2]], ['items' => [1, 2]]);\n        assert(2 + 2 === 4);\n    }\n    public function testExplainsAWrongTotal(): void {\n        Assert::equals(3, 1 + 1, 'the total');\n    }\n    public function testSeesAThrownError(): void {\n        $error = Assert::throws(fn() => intdiv(1, 0));\n        Assert::isTrue($error instanceof DivisionByZeroError);\n    }\n    public function testNeedsAThrownError(): void {\n        Assert::throws(fn() => 1);\n    }\n    #[ValueSource([1, 2, -3])]\n    public function testKeepsPositiveInputs(int $value): void {\n        assert($value > 0, 'a positive input');\n    }\n}\n";
    const DART_SETUP: &'static str = "@TestSetup\nclass CatalogTestSetup {\n  static BehaviorAdapters configure(BehaviorExecution execution) {\n    return BehaviorAdapters();\n  }\n}\n";
    const DART_UNIT: &'static str = "@Test(CatalogTestSetup)\nclass CatalogServiceTest {\n  Future<void> testAddsTwoNumbers() async {\n    Assert.equals(4, 2 + 2);\n    Assert.equals({'items': [1, 2]}, {'items': [1, 2]});\n    assert(2 + 2 == 4);\n  }\n\n  Future<void> testExplainsAWrongTotal() async {\n    Assert.equals(3, 1 + 1, 'the total');\n  }\n\n  Future<void> testSeesAThrownError() async {\n    final error = await Assert.throws(() => int.parse('x'));\n    Assert.isTrue(error is FormatException);\n    await Assert.throws(() async => throw StateError('later'));\n  }\n\n  Future<void> testNeedsAThrownError() async {\n    await Assert.throws(() => 1);\n  }\n\n  @ValueSource([1, 2, -3])\n  Future<void> testKeepsPositiveInputs(int value) async {\n    assert(value > 0, 'a positive input');\n  }\n}\n";
    const SWIFT_SETUP: &'static str = "@TestSetup\nfinal class CatalogTestSetup {\n  static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters {\n    BehaviorAdapters()\n  }\n}\n";
    const SWIFT_UNIT: &'static str = "@Test(CatalogTestSetup)\nfinal class CatalogServiceTest {\n  func testAddsTwoNumbers() async throws {\n    try Assert.equals(4, 2 + 2)\n    try Assert.equals([1, 2], [1, 2])\n    precondition(2 + 2 == 4)\n  }\n\n  func testExplainsAWrongTotal() async throws {\n    try Assert.equals(3, 1 + 1, \"the total\")\n  }\n\n  func testSeesAThrownError() async throws {\n    let error = try await Assert.throws { throw BehaviorError(\"EMPTY_NAME\") }\n    try Assert.isTrue(error is BehaviorError)\n  }\n\n  func testNeedsAThrownError() async throws {\n    try await Assert.throws {}\n  }\n\n  @ValueSource([1, 2, -3])\n  func testKeepsPositiveInputs(_ value: Int) async throws {\n    try Assert.isTrue(value > 0, \"a positive input\")\n  }\n}\n";
    fn write(root: &std::path::Path, relative: &str, source: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directories");
        std::fs::write(path, source).expect("source");
    }

    fn available(program: &str) -> bool {
        Self::answers(program, "--version")
    }

    fn answers(program: &str, flag: &str) -> bool {
        std::process::Command::new(program)
            .arg(flag)
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

    fn named_expected(
        names: [&'static str; 5],
        last_error: &'static str,
    ) -> [(String, bool, &'static str); 7] {
        let [adds, explains, sees, needs, positive] = names;
        [
            (adds.to_owned(), true, ""),
            (
                explains.to_owned(),
                false,
                "the total ==> expected: <3> but was: <2>",
            ),
            (sees.to_owned(), true, ""),
            (
                needs.to_owned(),
                false,
                "Expected an error to be thrown, but nothing was thrown.",
            ),
            (format!("{positive}[1]"), true, ""),
            (format!("{positive}[2]"), true, ""),
            (format!("{positive}[3]"), false, last_error),
        ]
    }

    fn check_named(
        extension: &str,
        cases: &[(String, bool, String)],
        expected: &[(String, bool, &'static str)],
    ) {
        let borrowed: Vec<(&str, bool, &str)> = expected
            .iter()
            .map(|(name, passed, error)| (name.as_str(), *passed, *error))
            .collect();
        Self::check(extension, cases, &borrowed);
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

    fn jvm_expected() -> [(&'static str, bool, &'static str); 7] {
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
            ("testKeepsPositiveInputs[3]", false, "a positive input"),
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

#[tokio::test]
async fn jvm_units_assert_with_the_helper_and_the_language_assert() {
    for (extension, program, setup, unit) in [
        (
            "java",
            "java",
            AssertedUnit::JAVA_SETUP,
            AssertedUnit::JAVA_UNIT,
        ),
        (
            "groovy",
            "groovyc",
            AssertedUnit::GROOVY_SETUP,
            AssertedUnit::GROOVY_UNIT,
        ),
        (
            "kt",
            "kotlinc",
            AssertedUnit::KOTLIN_SETUP,
            AssertedUnit::KOTLIN_UNIT,
        ),
    ] {
        if !AssertedUnit::answers(program, "-version") {
            eprintln!("skipping the {extension} unit: {program} is unavailable");
            continue;
        }
        let cases = AssertedUnit::server_cases(extension, setup, unit).await;
        AssertedUnit::check(extension, &cases, &AssertedUnit::jvm_expected());
    }
}

#[test]
fn a_tac_kotlin_unit_asserts_and_runs_one_case_per_input() {
    if !AssertedUnit::answers("kotlinc", "-version") {
        eprintln!("skipping: kotlinc is unavailable");
        return;
    }
    let cases =
        AssertedUnit::client_cases("kt", AssertedUnit::KOTLIN_SETUP, AssertedUnit::KOTLIN_UNIT);
    AssertedUnit::check("tac kt", &cases, &AssertedUnit::jvm_expected());
}

#[tokio::test]
async fn dotnet_units_assert_and_run_one_case_per_input() {
    if !AssertedUnit::available("dotnet") {
        eprintln!("skipping: dotnet is unavailable");
        return;
    }
    for (extension, setup, unit) in [
        ("cs", AssertedUnit::CSHARP_SETUP, AssertedUnit::CSHARP_UNIT),
        ("fs", AssertedUnit::FSHARP_SETUP, AssertedUnit::FSHARP_UNIT),
    ] {
        let yon = AssertedUnit::server_cases(extension, setup, unit).await;
        AssertedUnit::check(extension, &yon, &AssertedUnit::javascript_expected());
        let tac = AssertedUnit::client_cases(extension, setup, unit);
        AssertedUnit::check(
            &format!("tac {extension}"),
            &tac,
            &AssertedUnit::javascript_expected(),
        );
    }
}

struct NativeUnit {
    extension: &'static str,
    program: &'static str,
    flag: &'static str,
    setup: &'static str,
    unit: &'static str,
    names: [&'static str; 5],
    last_error: &'static str,
    yon: bool,
    tac: bool,
}

impl NativeUnit {
    const POSITIVE: &'static str = "a positive input ==> expected: <true> but was: <false>";
    const CAMEL_NAMES: [&'static str; 5] = [
        "testAddsTwoNumbers",
        "testExplainsAWrongTotal",
        "testSeesAThrownError",
        "testNeedsAThrownError",
        "testKeepsPositiveInputs",
    ];
    const UNITS: [Self; 5] = [
        Self {
            extension: "go",
            program: "go",
            flag: "version",
            setup: AssertedUnit::GO_SETUP,
            unit: AssertedUnit::GO_UNIT,
            names: [
                "TestAddsTwoNumbers",
                "TestExplainsAWrongTotal",
                "TestSeesAnError",
                "TestNeedsAnError",
                "TestKeepsPositiveInputs",
            ],
            last_error: Self::POSITIVE,
            yon: true,
            tac: true,
        },
        Self {
            extension: "rb",
            program: "ruby",
            flag: "--version",
            setup: AssertedUnit::RUBY_SETUP,
            unit: AssertedUnit::RUBY_UNIT,
            names: [
                "test_adds_two_numbers",
                "test_explains_a_wrong_total",
                "test_sees_a_raised_error",
                "test_needs_a_raised_error",
                "test_keeps_positive_inputs",
            ],
            last_error: Self::POSITIVE,
            yon: true,
            tac: true,
        },
        Self {
            extension: "php",
            program: "php-cgi",
            flag: "-v",
            setup: AssertedUnit::PHP_SETUP,
            unit: AssertedUnit::PHP_UNIT,
            names: Self::CAMEL_NAMES,
            last_error: "a positive input",
            yon: true,
            tac: false,
        },
        Self {
            extension: "dart",
            program: "dart",
            flag: "--version",
            setup: AssertedUnit::DART_SETUP,
            unit: AssertedUnit::DART_UNIT,
            names: Self::CAMEL_NAMES,
            last_error: "a positive input",
            yon: true,
            tac: true,
        },
        Self {
            extension: "swift",
            program: "swiftc",
            flag: "--version",
            setup: AssertedUnit::SWIFT_SETUP,
            unit: AssertedUnit::SWIFT_UNIT,
            names: Self::CAMEL_NAMES,
            last_error: Self::POSITIVE,
            yon: false,
            tac: true,
        },
    ];
}

#[tokio::test]
async fn go_ruby_php_dart_and_swift_units_assert_and_run_one_case_per_input() {
    for native in NativeUnit::UNITS {
        if !AssertedUnit::answers(native.program, native.flag) {
            eprintln!(
                "skipping the {} unit: {} is unavailable",
                native.extension, native.program
            );
            continue;
        }
        let expected = AssertedUnit::named_expected(native.names, native.last_error);
        if native.yon {
            let yon = AssertedUnit::server_cases(native.extension, native.setup, native.unit).await;
            AssertedUnit::check_named(native.extension, &yon, &expected);
        }
        if native.tac {
            let tac = AssertedUnit::client_cases(native.extension, native.setup, native.unit);
            AssertedUnit::check_named(&format!("tac {}", native.extension), &tac, &expected);
        }
    }
}
