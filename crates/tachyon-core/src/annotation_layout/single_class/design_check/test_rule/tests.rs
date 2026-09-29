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

    fn fsharp_messages(path: &str, text: &str) -> Vec<String> {
        let switches = DesignSwitches::every_realm(ProjectWords::default());
        let files = [(Path::new(path), text.as_bytes())];
        let found = DesignCheck::diagnostics(files.into_iter(), &switches);
        crate::fsharp_source::FsharpSpans::restored(&found, files.into_iter())
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
fn a_java_case_may_use_the_assert_statement() {
    let unit = "@Test(CatalogTestSetup.class)\nclass CatalogServiceTest {\n    int twice(int value) { return value * 2; }\n    void testDoublesAValue() {\n        Assert.equals(8, this.twice(4));\n    }\n    void testUsesTheStatement() {\n        assert this.twice(1) == 2 : \"doubles\";\n    }\n    void testSeesAnError() {\n        Assert.throwsError(() -> Integer.parseInt(\"x\"));\n    }\n    void testPicksALabel() {\n        Assert.equals(\"many\", 2 > 1 ? \"many\" : \"one\");\n    }\n    void testChecksEveryValue() {\n        for (int value : new int[] {1, 2}) {\n            Assert.isTrue(value > 0);\n        }\n    }\n    void testThrowsByHand() {\n        if (this.twice(4) != 8) throw new IllegalStateException(\"expected 8\");\n    }\n}\n";
    assert_eq!(
        TestedProject::messages(&[("server/tests/CatalogServiceTest.java", unit)]),
        [
            "The test case `testPicksALabel` in CatalogServiceTest holds logic: `? :`.",
            "The test case `testChecksEveryValue` in CatalogServiceTest holds logic: `for`.",
            "The test case `testThrowsByHand` in CatalogServiceTest holds logic: `if`.",
            "The test case `testThrowsByHand` in CatalogServiceTest has no assertion.",
        ]
    );
}

#[test]
fn a_kotlin_case_may_use_check_and_require() {
    let unit = "@Test(CatalogTestSetup::class)\nclass CatalogServiceTest {\n    fun twice(value: Int): Int = value * 2\n    fun testDoublesAValue() {\n        Assert.equals(8, twice(4))\n    }\n    fun testUsesCheck() {\n        check(twice(1) == 2)\n        require(twice(2) == 4) { \"doubles\" }\n    }\n    fun testSeesAnError() {\n        Assert.throws { \"x\".toInt() }\n    }\n    fun testPicksALabel() {\n        val label = when (twice(1)) { 2 -> \"two\" else -> \"other\" }\n        Assert.equals(\"two\", label)\n    }\n    fun testOnlyAsserts() {\n        assert(twice(4) == 8)\n    }\n}\n";
    assert_eq!(
        TestedProject::messages(&[("client/tests/CatalogServiceTest.kt", unit)]),
        [
            "The test case `testPicksALabel` in CatalogServiceTest holds logic: `when`.",
            "The test case `testOnlyAsserts` in CatalogServiceTest has no assertion.",
        ]
    );
}

#[test]
fn a_groovy_case_may_use_the_assert_statement() {
    let unit = "@Test(CatalogTestSetup.class)\nclass CatalogServiceTest {\n    int twice(int value) { return value * 2 }\n    void testDoublesAValue() {\n        Assert.equals(8, this.twice(4))\n        assert this.twice(1) == 2\n    }\n    void testSeesAnError() {\n        Assert.throwsError({ Integer.parseInt('x') })\n    }\n    void testChecksEveryValue() {\n        while (this.twice(1) < 0) {\n            Assert.isTrue(false)\n        }\n    }\n    void testThrowsByHand() {\n        if (this.twice(4) != 8) {\n            throw new IllegalStateException('expected 8')\n        }\n    }\n}\n";
    assert_eq!(
        TestedProject::messages(&[("server/tests/CatalogServiceTest.groovy", unit)]),
        [
            "The test case `testChecksEveryValue` in CatalogServiceTest holds logic: `while`.",
            "The test case `testThrowsByHand` in CatalogServiceTest holds logic: `if`.",
            "The test case `testThrowsByHand` in CatalogServiceTest has no assertion.",
        ]
    );
}

#[test]
fn a_csharp_case_asserts_with_the_helper_and_holds_no_logic() {
    let unit = "[Test(typeof(CatalogTestSetup), {\"partition\":\"tests\"})]\nsealed class CatalogServiceTest {\n    int twice(int value) { return value < 0 ? 0 : value * 2; }\n    void testDoublesAValue() {\n        Assert.equals(8, twice(4));\n    }\n    void testSeesAnError() {\n        var error = Assert.throws(() => int.Parse(\"x\"));\n        Assert.isTrue(error is System.FormatException);\n    }\n    void testReadsAnOptionalName() {\n        string name = null;\n        Assert.equals(0, name?.Length ?? 0);\n    }\n    void testPicksALabel() {\n        Assert.equals(\"many\", 2 > 1 ? \"many\" : \"one\");\n    }\n    void testChecksEveryValue() {\n        foreach (var value in new[] { 1, 2 }) {\n            Assert.isTrue(value > 0);\n        }\n    }\n    void testNamesAValue() {\n        var label = twice(1) switch { 2 => \"two\", _ => \"other\" };\n        Assert.equals(\"two\", label);\n    }\n    void testThrowsByHand() {\n        if (twice(4) != 8) throw new System.Exception(\"expected 8\");\n    }\n    void testOnlyDebugs() {\n        System.Diagnostics.Debug.Assert(twice(4) == 8);\n    }\n}\n";
    assert_eq!(
        TestedProject::messages(&[("client/tests/CatalogServiceTest.cs", unit)]),
        [
            "The test case `testPicksALabel` in CatalogServiceTest holds logic: `? :`.",
            "The test case `testChecksEveryValue` in CatalogServiceTest holds logic: `foreach`.",
            "The test case `testNamesAValue` in CatalogServiceTest holds logic: `switch`.",
            "The test case `testThrowsByHand` in CatalogServiceTest holds logic: `if`.",
            "The test case `testThrowsByHand` in CatalogServiceTest has no assertion.",
            "The test case `testOnlyDebugs` in CatalogServiceTest has no assertion.",
        ]
    );
}

#[test]
fn an_fsharp_case_asserts_with_the_helper_and_holds_no_logic() {
    let unit = "[<Test(typeof<CatalogTestSetup>)>]\ntype CatalogServiceTest() =\n    member this.Twice(value: int) : int =\n        if value < 0 then 0 else value * 2\n\n    member this.testDoublesAValue() : unit =\n        Assert.equals(8, this.Twice(4))\n\n    member this.testSeesAnError() : unit =\n        Assert.throws(fun () -> int \"x\") |> ignore\n\n    member this.testPicksALabel() : unit =\n        let label = match this.Twice(1) with | 2 -> \"two\" | _ -> \"other\"\n        Assert.equals(\"two\", label)\n\n    member this.testChecksEveryValue() : unit =\n        for value in [1; 2] do\n            Assert.isTrue(value > 0)\n\n    member this.testWaits() : unit =\n        while this.Twice(1) < 0 do\n            Assert.isTrue(false)\n\n    member this.testThrowsByHand() : unit =\n        if this.Twice(4) <> 8 then failwith \"expected 8\"\n";
    assert_eq!(
        TestedProject::fsharp_messages("server/tests/CatalogServiceTest.fs", unit),
        [
            "The test case `testPicksALabel` in CatalogServiceTest holds logic: `match`.",
            "The test case `testChecksEveryValue` in CatalogServiceTest holds logic: `for`.",
            "The test case `testWaits` in CatalogServiceTest holds logic: `while`.",
            "The test case `testThrowsByHand` in CatalogServiceTest holds logic: `if`.",
            "The test case `testThrowsByHand` in CatalogServiceTest has no assertion.",
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
fn the_rule_is_on_unless_a_realm_turns_it_off() {
    let files = [("server/tests/CatalogServiceTest.js", JAVASCRIPT_UNIT)];
    assert_eq!(TestedProject::with_settings(&files, &[]).len(), 3);
    let turned_off = [("server/application.json", "{\"tests\":{\"check\":false}}")];
    assert!(TestedProject::with_settings(&files, &turned_off).is_empty());
    let client_only = [(
        "client/manifest.json",
        "{\"tac\":{\"tests\":{\"check\":false}}}",
    )];
    assert_eq!(TestedProject::with_settings(&files, &client_only).len(), 3);
}

#[test]
fn a_go_case_asserts_with_the_helper_and_holds_no_logic() {
    let unit = "// @Test(CatalogTestSetup, {\"partition\":\"tests\"})\ntype CatalogServiceTest struct {\n\tbehaviors *BehaviorContext\n}\n\nfunc (test *CatalogServiceTest) twice(value int) int {\n\tif value < 0 {\n\t\treturn 0\n\t}\n\treturn value * 2\n}\n\nfunc (test *CatalogServiceTest) TestDoublesAValue() {\n\tAssert.Equals(8, test.twice(4))\n}\n\nfunc (test *CatalogServiceTest) TestSeesAnError() {\n\tAssert.Throws(func() error { return errors.New(\"no\") })\n}\n\nfunc (test *CatalogServiceTest) TestChecksEveryValue() {\n\tfor _, value := range []int{1, 2} {\n\t\tAssert.IsTrue(value > 0)\n\t}\n}\n\nfunc (test *CatalogServiceTest) TestPicksALabel() {\n\tswitch test.twice(1) {\n\tcase 2:\n\t\tAssert.IsTrue(true)\n\t}\n}\n\nfunc (test *CatalogServiceTest) TestReturnsAnError() error {\n\tif test.twice(4) != 8 {\n\t\treturn errors.New(\"expected 8\")\n\t}\n\treturn nil\n}\n";
    assert_eq!(
        TestedProject::messages(&[("server/tests/CatalogServiceTest.go", unit)]),
        [
            "The test case `TestChecksEveryValue` in CatalogServiceTest holds logic: `for`.",
            "The test case `TestPicksALabel` in CatalogServiceTest holds logic: `switch`.",
            "The test case `TestReturnsAnError` in CatalogServiceTest holds logic: `if`.",
            "The test case `TestReturnsAnError` in CatalogServiceTest has no assertion.",
        ]
    );
}

#[test]
fn a_ruby_case_asserts_with_the_helper_and_holds_no_logic() {
    let unit = "#[Test(CatalogTestSetup, {\"partition\":\"tests\"})]\nclass CatalogServiceTest\n  def twice(value)\n    return 0 if value < 0\n    value * 2\n  end\n\n  def test_doubles_a_value\n    Assert.equals(8, twice(4))\n  end\n\n  def test_sees_an_error\n    Assert.throws { Integer(\"x\") }\n  end\n\n  def test_picks_a_label\n    Assert.equals(\"many\", 2 > 1 ? \"many\" : \"one\")\n  end\n\n  def test_raises_by_hand\n    raise \"expected 8\" unless twice(4) == 8\n  end\n\n  def test_waits\n    Assert.is_true(true) until twice(1) > 0\n  end\n\n  def test_names_a_value\n    label = case twice(1)\n            when 2 then \"two\"\n            end\n    Assert.equals(\"two\", label)\n  end\nend\n";
    assert_eq!(
        TestedProject::messages(&[("client/tests/CatalogServiceTest.rb", unit)]),
        [
            "The test case `test_picks_a_label` in CatalogServiceTest holds logic: `? :`.",
            "The test case `test_raises_by_hand` in CatalogServiceTest holds logic: `unless`.",
            "The test case `test_raises_by_hand` in CatalogServiceTest has no assertion.",
            "The test case `test_waits` in CatalogServiceTest holds logic: `until`.",
            "The test case `test_names_a_value` in CatalogServiceTest holds logic: `case`.",
        ]
    );
}

#[test]
fn a_php_case_may_use_the_assert_function() {
    let unit = "<?php\n#[Test(CatalogTestSetup::class, {\"partition\":\"tests\"})]\nclass CatalogServiceTest {\n    private function twice(int $value): int { return $value < 0 ? 0 : $value * 2; }\n    public function testDoublesAValue(): void {\n        Assert::equals(8, $this->twice(4));\n    }\n    public function testUsesTheFunction(): void {\n        assert($this->twice(1) === 2);\n    }\n    public function testSeesAnError(): void {\n        Assert::throws(fn() => intdiv(1, 0));\n    }\n    public function testPicksALabel(): void {\n        Assert::equals('many', 2 > 1 ? 'many' : 'one');\n    }\n    public function testNamesAValue(): void {\n        $label = match ($this->twice(1)) { 2 => 'two', default => 'other' };\n        Assert::equals('two', $label);\n    }\n    public function testChecksEveryValue(): void {\n        foreach ([1, 2] as $value) {\n            Assert::isTrue($value > 0);\n        }\n    }\n    public function testThrowsByHand(): void {\n        if ($this->twice(4) !== 8) throw new Exception('expected 8');\n    }\n}\n";
    assert_eq!(
        TestedProject::messages(&[("server/tests/CatalogServiceTest.php", unit)]),
        [
            "The test case `testPicksALabel` in CatalogServiceTest holds logic: `? :`.",
            "The test case `testNamesAValue` in CatalogServiceTest holds logic: `match`.",
            "The test case `testChecksEveryValue` in CatalogServiceTest holds logic: `foreach`.",
            "The test case `testThrowsByHand` in CatalogServiceTest holds logic: `if`.",
            "The test case `testThrowsByHand` in CatalogServiceTest has no assertion.",
        ]
    );
}

#[test]
fn a_dart_case_may_use_the_assert_statement() {
    let unit = "@Test(CatalogTestSetup, {\"partition\":\"tests\"})\nclass CatalogServiceTest {\n  int twice(int value) => value < 0 ? 0 : value * 2;\n\n  Future<void> testDoublesAValue() async {\n    Assert.equals(8, twice(4));\n  }\n\n  Future<void> testUsesTheStatement() async {\n    assert(twice(1) == 2, 'doubles');\n  }\n\n  Future<void> testSeesAnError() async {\n    await Assert.throws(() => int.parse('x'));\n  }\n\n  Future<void> testReadsAnOptionalName() async {\n    String? name;\n    Assert.equals(0, name?.length ?? 0);\n  }\n\n  Future<void> testPicksALabel() async {\n    Assert.equals('many', 2 > 1 ? 'many' : 'one');\n  }\n\n  Future<void> testChecksEveryValue() async {\n    for (final value in [1, 2]) {\n      Assert.isTrue(value > 0);\n    }\n  }\n\n  Future<void> testThrowsByHand() async {\n    if (twice(4) != 8) throw StateError('expected 8');\n  }\n}\n";
    assert_eq!(
        TestedProject::messages(&[("server/tests/CatalogServiceTest.dart", unit)]),
        [
            "The test case `testPicksALabel` in CatalogServiceTest holds logic: `? :`.",
            "The test case `testChecksEveryValue` in CatalogServiceTest holds logic: `for`.",
            "The test case `testThrowsByHand` in CatalogServiceTest holds logic: `if`.",
            "The test case `testThrowsByHand` in CatalogServiceTest has no assertion.",
        ]
    );
}

#[test]
fn a_swift_case_may_use_precondition() {
    let unit = "@Test(CatalogTestSetup, {\"partition\":\"tests\"})\nfinal class CatalogServiceTest {\n  func twice(_ value: Int) -> Int { value < 0 ? 0 : value * 2 }\n\n  func testDoublesAValue() async throws {\n    try Assert.equals(8, twice(4))\n  }\n\n  func testUsesPrecondition() async throws {\n    precondition(twice(1) == 2, \"doubles\")\n  }\n\n  func testSeesAnError() async throws {\n    try await Assert.throws { throw BehaviorError(\"no\") }\n  }\n\n  func testReadsAnOptionalName() async throws {\n    let name: String? = nil\n    try Assert.equals(0, name?.count ?? 0)\n  }\n\n  func testPicksALabel() async throws {\n    try Assert.equals(\"many\", 2 > 1 ? \"many\" : \"one\")\n  }\n\n  func testGuards() async throws {\n    guard twice(4) == 8 else { throw BehaviorError(\"expected 8\") }\n  }\n\n  func testOnlyAsserts() async throws {\n    assert(twice(4) == 8)\n  }\n}\n";
    assert_eq!(
        TestedProject::messages(&[("client/tests/CatalogServiceTest.swift", unit)]),
        [
            "The test case `testPicksALabel` in CatalogServiceTest holds logic: `? :`.",
            "The test case `testGuards` in CatalogServiceTest holds logic: `guard`.",
            "The test case `testGuards` in CatalogServiceTest has no assertion.",
            "The test case `testOnlyAsserts` in CatalogServiceTest has no assertion.",
        ]
    );
}
