#![allow(clippy::expect_used)]

use super::*;
use std::path::Path;

fn write(root: &Path, relative: &str, source: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("directories");
    std::fs::write(path, source).expect("source");
}

pub(super) fn report_with(files: &[(&str, &str)], calls: RecordedCalls) -> CoverageReport {
    let root = tempfile::tempdir().expect("project");
    for (relative, source) in files {
        write(root.path(), relative, source);
    }
    CoverageReport::build(root.path(), calls).expect("report")
}

fn report_for(files: &[(&str, &str)]) -> CoverageReport {
    report_with(files, RecordedCalls::default())
}

pub(super) fn uncovered_methods(report: &CoverageReport) -> Vec<String> {
    report
        .uncovered()
        .into_iter()
        .map(|entry| entry.method)
        .collect()
}

fn calls(realm: &str, case: &str, called: &[&str]) -> RecordedCalls {
    let mut calls = RecordedCalls::default();
    let called: Vec<String> = called.iter().map(|key| (*key).to_owned()).collect();
    calls.add(realm, case, &called);
    calls
}

const CATALOG_SERVICE: &str =
    "@Service()\nclass CatalogService {\n  total() { return 2; }\n  untested() { return 3; }\n}\n";
const CATALOG_UNIT: &str = "@Test(C)\nexport default class CatalogServiceTest {\n  async testTotals() { const service = this.construct(CatalogService); service.total(); if (false) service.untested(); }\n}\n";

#[test]
fn a_method_a_case_names_but_never_calls_is_not_covered() {
    let report = report_with(
        &[
            ("server/services/CatalogService.js", CATALOG_SERVICE),
            ("server/tests/CatalogServiceTest.js", CATALOG_UNIT),
        ],
        calls(
            "server",
            "CatalogServiceTest.testTotals",
            &["CatalogService.total"],
        ),
    );
    assert_eq!(report.covered(), 1, "{report:?}");
    assert_eq!(
        report.uncovered(),
        vec![Uncovered {
            file: String::from("server/services/CatalogService.js"),
            type_name: String::from("CatalogService"),
            method: String::from("untested"),
            recorded: true,
        }],
        "{report:?}"
    );
}

#[test]
fn a_called_method_names_the_first_case_that_called_it() {
    let mut recorded = calls(
        "server",
        "CatalogServiceTest.testTotals",
        &["CatalogService.total"],
    );
    recorded.add(
        "server",
        "CatalogServiceTest.testAgain",
        &[
            String::from("CatalogService.total"),
            String::from("CatalogService.untested"),
        ],
    );
    let report = report_with(
        &[("server/services/CatalogService.js", CATALOG_SERVICE)],
        recorded,
    );
    let states: Vec<MethodState> = report.classes[0]
        .methods
        .iter()
        .map(|method| method.state.clone())
        .collect();
    assert_eq!(
        states,
        vec![
            MethodState::Called(String::from("CatalogServiceTest.testTotals")),
            MethodState::Called(String::from("CatalogServiceTest.testAgain")),
        ]
    );
    let details = CoverageText::details(&report).join("\n");
    assert!(
        details.contains("called   total (first by CatalogServiceTest.testTotals)"),
        "{details}"
    );
}

#[test]
fn a_call_counts_only_in_the_realm_whose_test_made_it() {
    let report = report_with(
        &[("server/services/CatalogService.js", CATALOG_SERVICE)],
        calls(
            "client",
            "CatalogServiceTest.testTotals",
            &["CatalogService.total", "CatalogService.untested"],
        ),
    );
    assert_eq!(
        uncovered_methods(&report),
        vec!["total", "untested"],
        "{report:?}"
    );
}

#[test]
fn a_missing_method_says_no_test_case_called_it() {
    let report = report_for(&[("client/services/CatalogService.ts", CATALOG_SERVICE)]);
    let missing = CoverageText::missing(&report).join("\n");
    assert!(
        missing.contains(
            "no test case called CatalogService.total (client/services/CatalogService.ts)"
        ),
        "{missing}"
    );
}

#[test]
fn a_language_whose_units_could_not_run_falls_back_to_the_labelled_name_check() {
    let mut recorded = RecordedCalls::default();
    recorded.add_unit("client", "swift", false);
    let service = "@Service()\nfinal class CatalogService {\n    func total() -> Int { 2 }\n    func untested() -> Int { 3 }\n}\n";
    let unit = "@Test(C)\nfinal class CatalogServiceTest {\n    func testTotals() async throws {\n        _ = CatalogService().total()\n    }\n}\n";
    let report = report_with(
        &[
            ("client/services/CatalogService.swift", service),
            ("client/tests/CatalogServiceTest.swift", unit),
        ],
        recorded.clone(),
    );
    assert!(!report.classes[0].recorded, "{report:?}");
    assert_eq!(report.classes[0].methods[0].state, MethodState::Named);
    assert_eq!(uncovered_methods(&report), vec!["untested"]);
    let details = CoverageText::details(&report).join("\n");
    assert!(
        details.contains("[name check: no .swift test unit could run here"),
        "{details}"
    );
    recorded.add_unit("client", "swift", true);
    let ran = report_with(
        &[
            ("client/services/CatalogService.swift", service),
            ("client/tests/CatalogServiceTest.swift", unit),
        ],
        recorded,
    );
    assert!(ran.classes[0].recorded, "{ran:?}");
    assert_eq!(uncovered_methods(&ran), vec!["total", "untested"]);
}

#[test]
fn a_call_is_not_a_public_method() {
    let report = report_for(&[(
        "server/services/HeroService.js",
        "@Service()\nclass HeroService {\n  copy() {\n    clearTimeout(this.timer);\n    this.timer = setTimeout(() => {\n      this.label = encodeURIComponent('x');\n    }, 2000);\n  }\n}\n",
    )]);
    assert_eq!(uncovered_methods(&report), vec!["copy"], "{report:?}");
}

#[test]
fn an_unannotated_class_owes_nothing() {
    let report = report_for(&[(
        "server/services/PlainService.js",
        "class PlainService {\n  total() { return 2; }\n}\n",
    )]);
    assert_eq!(report.total(), 0);
}

#[test]
fn the_compiler_writes_a_declared_method_so_nobody_tests_it() {
    let report = report_for(&[(
        "server/stores/OrderRepository.java",
        "@Store(Order.class)\nclass OrderRepository {\n    java.util.List<Order> findBySku(String sku);\n    public int authored() { return 1; }\n}\n",
    )]);
    assert_eq!(uncovered_methods(&report), vec!["authored"], "{report:?}");
}

#[test]
fn a_kotlin_http_client_declares_its_requests_so_nobody_tests_them() {
    let report = report_for(&[(
        "client/remotes/GreetingClient.kt",
        "@HTTPClient(\"/api\")\ninterface GreetingClient {\n    @GetRequest(\"/greeting\")\n    suspend fun getGreeting(): Greeting\n    fun read(\n        id: Int,\n    ): Greeting = getGreeting()\n}\n",
    )]);
    assert_eq!(uncovered_methods(&report), vec!["read"], "{report:?}");
}

#[test]
fn a_swift_http_client_declares_its_requests_so_nobody_tests_them() {
    let report = report_for(&[(
        "client/remotes/GreetingClient.swift",
        "@HTTPClient(\"/api\")\nprotocol GreetingClient {\n    @GetRequest(\"/greeting\")\n    func getGreeting() async throws -> Greeting\n    func read(\n        id: Int\n    ) async throws -> Greeting {\n        try await getGreeting()\n    }\n}\n",
    )]);
    assert_eq!(uncovered_methods(&report), vec!["read"], "{report:?}");
}

#[test]
fn a_ruby_http_client_declares_its_requests_so_nobody_tests_them() {
    let report = report_for(&[(
        "client/remotes/GreetingClient.rb",
        "#[HTTPClient(\"/api\")]\nclass GreetingClient\n  # @return [Greeting]\n  #[GetRequest(\"/greeting\")]\n  def get_greeting\n  end\n\n  # @param page [Integer]\n  # @return [Hash]\n  #[GetRequest(\"/products\")]\n  def all(\n    #[RequestParam(\"page\")]\n    page\n  )\n  end\n\n  def read(id)\n    get_greeting\n  end\nend\n",
    )]);
    assert_eq!(uncovered_methods(&report), vec!["read"], "{report:?}");
}

#[test]
fn a_lua_http_client_declares_its_requests_so_nobody_tests_them() {
    let report = report_for(&[(
        "client/remotes/GreetingClient.lua",
        "-- @HTTPClient(\"/api\")\n---@class GreetingClient\nGreetingClient = {}\n\n---@return Greeting\n-- @GetRequest(\"/greeting\")\nfunction GreetingClient:read()\nend\n\n---@param id integer\n---@param page integer\n---@return Greeting\n-- @GetRequest(\"/greeting/{id}\")\nfunction GreetingClient:find(\n  -- @PathVariable(\"id\")\n  id,\n  -- @RequestParam(\"page\")\n  page\n)\nend\n\n---@return string\nfunction GreetingClient:label()\n  return self:read().message\nend\n\n---@return string\nfunction GreetingClient:endless()\n  return \"\"\nend\n",
    )]);
    assert_eq!(
        uncovered_methods(&report),
        vec!["label", "endless"],
        "{report:?}"
    );
}

#[test]
fn a_ruby_method_that_only_returns_a_string_still_needs_a_test() {
    let report = report_for(&[(
        "server/services/GreetingService.rb",
        "#[Service]\nclass GreetingService\n  # @return [String]\n  def message\n    \"Hello\"\n  end\n\n  def silent\n  end\nend\n",
    )]);
    assert_eq!(uncovered_methods(&report), vec!["message"], "{report:?}");
}

#[test]
fn recorded_calls_survive_a_write_and_a_read() {
    let root = tempfile::tempdir().expect("project");
    assert_eq!(RecordedCalls::read(root.path()), None);
    let recorded = calls(
        "client",
        "CatalogServiceTest.testTotals",
        &["CatalogService.total"],
    );
    recorded.write(root.path()).expect("write");
    assert_eq!(RecordedCalls::read(root.path()), Some(recorded));
}

#[test]
fn a_record_older_than_a_source_is_not_current() {
    let root = tempfile::tempdir().expect("project");
    write(
        root.path(),
        "server/services/CatalogService.js",
        CATALOG_SERVICE,
    );
    let mut recorded = calls(
        "server",
        "CatalogServiceTest.testTotals",
        &["CatalogService.total"],
    );
    recorded.set_successful(true);
    std::thread::sleep(std::time::Duration::from_millis(20));
    recorded.write(root.path()).expect("write");
    assert_eq!(RecordedCalls::read_current(root.path()), Some(recorded));
    std::thread::sleep(std::time::Duration::from_millis(20));
    write(
        root.path(),
        "server/tests/CatalogServiceTest.js",
        CATALOG_UNIT,
    );
    assert_eq!(RecordedCalls::read_current(root.path()), None);
}

#[test]
fn a_failed_run_keeps_its_calls_but_is_not_current() {
    let root = tempfile::tempdir().expect("project");
    let recorded = calls("server", "CatalogTest.testTotal", &["CatalogService.total"]);
    recorded.write(root.path()).expect("write");
    assert_eq!(RecordedCalls::read(root.path()), Some(recorded));
    assert_eq!(RecordedCalls::read_current(root.path()), None);
}

#[test]
fn starting_a_new_run_invalidates_the_previous_successful_record() {
    let root = tempfile::tempdir().expect("project");
    let mut recorded = calls("server", "CatalogTest.testTotal", &["CatalogService.total"]);
    recorded.set_successful(true);
    recorded.write(root.path()).expect("write");
    assert_eq!(
        RecordedCalls::read_current(root.path()),
        Some(recorded.clone())
    );
    RecordedCalls::invalidate(root.path()).expect("invalidate");
    assert_eq!(RecordedCalls::read_current(root.path()), None);
    recorded.set_successful(false);
    assert_eq!(RecordedCalls::read(root.path()), Some(recorded));
}

#[test]
fn a_legacy_record_without_a_run_result_is_not_current() {
    let root = tempfile::tempdir().expect("project");
    write(
        root.path(),
        ".tachyon/coverage/calls.json",
        r#"{"realms":{},"languages":{}}"#,
    );
    assert!(RecordedCalls::read(root.path()).is_some());
    assert_eq!(RecordedCalls::read_current(root.path()), None);
}

#[test]
fn an_unwritable_record_refuses_invalidation() {
    let root = tempfile::tempdir().expect("project");
    std::fs::create_dir_all(RecordedCalls::path(root.path())).expect("blocking directory");
    let failure = RecordedCalls::invalidate(root.path()).expect_err("invalid record path");
    assert_eq!(failure.diagnostics()[0].code.to_string(), "TY2024");
}

fn hooked(relative: &str, source: &str) -> String {
    CallHooks::insert(Path::new(relative), source).expect("hooks")
}

#[test]
fn a_test_build_records_each_public_method_as_it_starts() {
    let output = hooked(
        "client/services/CatalogService.ts",
        "@Service()\nexport default class CatalogService {\n  total() { return 2 }\n  async load(): Promise<{ items: string[] }> {\n    return { items: [] }\n  }\n  private hidden(): number { return 3 }\n  static build() { return 4 }\n}\n",
    );
    assert!(
        output.contains("total() { Behaviors.called(\"CatalogService.total\"); return 2 }"),
        "{output}"
    );
    assert!(
        output.contains(
            "Promise<{ items: string[] }> { Behaviors.called(\"CatalogService.load\");\n"
        ),
        "{output}"
    );
    assert!(
        output.contains("private hidden(): number { return 3 }"),
        "{output}"
    );
    assert!(output.contains("static build() { return 4 }"), "{output}");
}

#[test]
fn a_utility_records_its_static_methods() {
    let output = hooked(
        "client/shared/utils/PriceUtils.js",
        "export default class PriceUtils {\n  static label(cents) {\n    return cents / 100\n  }\n}\n",
    );
    assert!(
        output.contains("static label(cents) { Behaviors.called(\"PriceUtils.label\");\n"),
        "{output}"
    );
}

#[test]
fn only_role_classes_in_a_test_build_get_hooks() {
    let unit = "@Test(C)\nexport default class CatalogServiceTest {\n  async testTotals() { return 1 }\n}\n";
    assert_eq!(hooked("client/tests/CatalogServiceTest.js", unit), unit);
    let service = "@Service()\nclass CatalogService {\n  total() { return 2; }\n}\n";
    let production = CallHooks::for_entry(
        Path::new("client/pages/tac.js"),
        Path::new("client/services/CatalogService.js"),
        service.to_owned(),
    )
    .expect("production");
    assert_eq!(production, service);
    let tested = CallHooks::for_entry(
        Path::new("client/tests/CatalogServiceTest.js"),
        Path::new("client/services/CatalogService.js"),
        service.to_owned(),
    )
    .expect("test build");
    assert!(
        tested.contains("Behaviors.called(\"CatalogService.total\")"),
        "{tested}"
    );
}

#[test]
fn a_python_method_records_its_call_on_its_first_body_line() {
    let output = hooked(
        "server/services/CatalogService.py",
        "@service()\nclass CatalogService:\n    def total(self, items: dict[str, int] = {}) -> int:\n        \"\"\"Count.\"\"\"\n        return 2\n\n    async def load(self): return []\n\n    def _hidden(self):\n        return 3\n",
    );
    assert!(
        output.contains(
            "-> int:\n        BehaviorCalls.record(\"CatalogService.total\")\n        \"\"\"Count."
        ),
        "{output}"
    );
    assert!(
        output.contains(
            "async def load(self): BehaviorCalls.record(\"CatalogService.load\"); return []"
        ),
        "{output}"
    );
    assert!(
        output.contains("def _hidden(self):\n        return 3"),
        "{output}"
    );
}

#[test]
fn a_ruby_method_records_its_call_after_its_signature() {
    let output = hooked(
        "server/services/CatalogService.rb",
        "#[Service]\nclass CatalogService\n  def total(items = [])\n    2\n  end\n\n  def count = 3\n\n  def label name\n    name\n  end\n\n  #[Cached]\n  def price(\n    #[CacheKey]\n    sku,\n    note\n  )\n    sku\n  end\n\n  private\n\n  def hidden\n    3\n  end\nend\n",
    );
    assert!(
        output.contains("    note\n  ); BehaviorCalls.record(\"CatalogService.price\")\n    sku\n"),
        "{output}"
    );
    assert!(
        output.contains("def total(items = []); BehaviorCalls.record(\"CatalogService.total\")\n"),
        "{output}"
    );
    assert!(
        output.contains("def count = (BehaviorCalls.record(\"CatalogService.count\"); 3)\n"),
        "{output}"
    );
    assert!(
        output.contains("def label name; BehaviorCalls.record(\"CatalogService.label\")\n"),
        "{output}"
    );
    assert!(output.contains("def hidden\n    3"), "{output}");
}

#[test]
fn a_lua_method_records_its_call_after_its_parameters_on_the_same_line() {
    let output = hooked(
        "server/controllers/CatalogController.lua",
        "-- @RequestRoute(\"/catalog\")\n-- @Public\n---@class CatalogController\nCatalogController = {}\n\n---@return integer\n-- @GetRoute\nfunction CatalogController:total()\n  return 2\nend\n\n---@param id integer\n---@return integer\n-- @GetRoute(\"/{id}\")\nfunction CatalogController:find(\n  -- @FromPath\n  id\n)\n  return id\nend\n\n---@return integer\nfunction CatalogController:_hidden()\n  return 3\nend\n",
    );
    assert!(
        output.contains("function CatalogController:total() BehaviorCalls.record(\"CatalogController.total\");\n  return 2\n"),
        "{output}"
    );
    assert!(
        output.contains("  id\n) BehaviorCalls.record(\"CatalogController.find\");\n  return id\n"),
        "{output}"
    );
    assert!(
        output.contains("function CatalogController:_hidden()\n  return 3"),
        "{output}"
    );
}

#[test]
fn a_php_method_records_its_call_even_with_its_brace_on_the_next_line() {
    let output = hooked(
        "server/services/CatalogService.php",
        "<?php\n#[Service]\nclass CatalogService {\n    public function total(): int\n    {\n        return 2;\n    }\n    private function hidden(): int { return 3; }\n}\n",
    );
    assert!(
        output.contains("    { \\BehaviorCalls::record('CatalogService.total');\n"),
        "{output}"
    );
    assert!(
        output.contains("private function hidden(): int { return 3; }"),
        "{output}"
    );
}

#[test]
fn a_kotlin_expression_body_keeps_its_value() {
    let output = hooked(
        "server/services/CatalogService.kt",
        "@Service\nclass CatalogService {\n    fun total(): Int = 2\n    fun load(): List<Int> {\n        return listOf()\n    }\n    private fun hidden(): Int = 3\n}\n",
    );
    assert!(
        output.contains(
            "fun total(): Int = BehaviorCalls.record(\"CatalogService.total\") behaviorThen 2\n"
        ),
        "{output}"
    );
    assert!(
        output.contains("fun load(): List<Int> { BehaviorCalls.record(\"CatalogService.load\");\n"),
        "{output}"
    );
    assert!(output.contains("private fun hidden(): Int = 3"), "{output}");
}

#[test]
fn a_dart_arrow_body_becomes_a_block_that_records_first() {
    let output = hooked(
        "client/services/CatalogService.dart",
        "@Service()\nclass CatalogService {\n  int total() => [1, 2].length;\n  void reset() => _count = 0;\n  Future<List<int>> load() async {\n    return [];\n  }\n  int _count = 0;\n}\n",
    );
    assert!(
        output.contains(
            "int total() { BehaviorCalls.record(r'CatalogService.total'); return [1, 2].length; }"
        ),
        "{output}"
    );
    assert!(
        output.contains(
            "void reset() { BehaviorCalls.record(r'CatalogService.reset'); _count = 0; }"
        ),
        "{output}"
    );
    assert!(
        output.contains("async { BehaviorCalls.record(r'CatalogService.load');\n"),
        "{output}"
    );
}

#[test]
fn a_swift_single_expression_keeps_its_value() {
    let output = hooked(
        "client/services/CatalogService.swift",
        "@Service()\nfinal class CatalogService {\n    func total() -> Int { 2 }\n    func label() -> String {\n        \"a\"\n    }\n    func reset() { count = 0 }\n    func load() -> [Int] {\n        let items = [1]\n        return items\n    }\n    var count = 0\n}\n",
    );
    assert!(
        output.contains(
            "func total() -> Int { BehaviorCalls.record(\"CatalogService.total\"); return 2 }"
        ),
        "{output}"
    );
    assert!(
        output.contains("BehaviorCalls.record(\"CatalogService.label\");\n        return \"a\""),
        "{output}"
    );
    assert!(
        output
            .contains("func reset() { BehaviorCalls.record(\"CatalogService.reset\"); count = 0 }"),
        "{output}"
    );
    assert!(
        output.contains("{ BehaviorCalls.record(\"CatalogService.load\");\n        let items"),
        "{output}"
    );
}

#[test]
fn compiled_languages_record_through_their_own_recorder() {
    for (relative, source, expected) in [
        (
            "server/services/CatalogService.java",
            "@Service\nclass CatalogService {\n    public int total() {\n        return 2;\n    }\n}\n",
            "public int total() { BehaviorCalls.record(\"CatalogService.total\");",
        ),
        (
            "server/services/CatalogService.cs",
            "[Service]\nsealed class CatalogService {\n    public int Total() { return 2; }\n}\n",
            "public int Total() { BehaviorCalls.Record(\"CatalogService.Total\"); return 2; }",
        ),
        (
            "server/services/CatalogService.go",
            "// @Service()\ntype CatalogService struct{}\n\nfunc (service *CatalogService) Total() int {\n\treturn 2\n}\n",
            "Total() int { behaviorCallsRecord(\"CatalogService.Total\");\n",
        ),
        (
            "server/services/CatalogService.rs",
            "#[Service()]\nstruct CatalogService {}\nimpl CatalogService {\n    fn total(&self) -> i32 { 2 }\n}\n",
            "fn total(&self) -> i32 { BehaviorCalls::record(\"CatalogService.total\"); 2 }",
        ),
    ] {
        let output = hooked(relative, source);
        assert!(output.contains(expected), "{relative}: {output}");
    }
}

#[test]
fn every_language_reports_its_own_public_methods() {
    for (extension, source, expected) in [
        (
            "ts",
            "@Service()\nexport default class CatalogService {\n  total(): number {\n    return 2\n  }\n  async load(): Promise<{ items: string[] }> {\n    return { items: [] }\n  }\n  private hidden(): number {\n    return 3\n  }\n}\n",
            vec!["total", "load"],
        ),
        (
            "py",
            "@service()\nclass CatalogService:\n    def total(self):\n        return 2\n    def _hidden(self):\n        return 3\n",
            vec!["total"],
        ),
        (
            "php",
            "<?php\n#[Service]\nclass CatalogService {\n    public function total(): int { return 2; }\n    private function hidden(): int { return 3; }\n}\n",
            vec!["total"],
        ),
        (
            "kt",
            "@Service\nclass CatalogService {\n    fun total(): Int = 2\n    private fun hidden(): Int = 3\n}\n",
            vec!["total"],
        ),
        (
            "swift",
            "@Service()\nfinal class CatalogService {\n    func total() -> Int { 2 }\n    private func hidden() -> Int { 3 }\n}\n",
            vec!["total"],
        ),
        (
            "cs",
            "[Service]\nsealed class CatalogService {\n    public int total() { return 2; }\n    private int hidden() { return 3; }\n}\n",
            vec!["total"],
        ),
        (
            "rs",
            "#[Service()]\nstruct CatalogService {}\nimpl CatalogService {\n    fn total(&self) -> i32 { 2 }\n    fn new() -> Self { Self {} }\n}\n",
            vec!["total"],
        ),
        (
            "go",
            "// @Service()\ntype CatalogService struct{}\n\nfunc (service *CatalogService) Total() int {\n\treturn 2\n}\n\nfunc (service *CatalogService) hidden() int { return 3 }\n\nfunc (service *CatalogService) CountByName(name BehaviorValue) (BehaviorValue, error) {}\n",
            vec!["Total"],
        ),
        (
            "rb",
            "#[Service]\nclass CatalogService\n  def initialize\n    @count = 2\n  end\n\n  def total\n    @count\n  end\n\n  def self.build\n    new\n  end\n\n  private\n\n  def hidden\n    3\n  end\nend\n",
            vec!["total"],
        ),
        (
            "lua",
            "-- @Service\n---@class CatalogService\nCatalogService = {\n  ---@type integer\n  count = 2,\n}\n\n---@return integer\nfunction CatalogService:total()\n  return self.count\nend\n\n---@return integer\nfunction CatalogService:_hidden()\n  return 3\nend\n",
            vec!["total"],
        ),
        (
            "dart",
            "@Service()\nclass CatalogService {\n  int total() => 2;\n  Future<Object?> load() async {\n    return null;\n  }\n  int _hidden() => 3;\n  static int build() => 4;\n  Future<Object?> countByName(Object? name) async {}\n}\n",
            vec!["total", "load"],
        ),
    ] {
        let relative = format!("server/services/CatalogService.{extension}");
        let report = report_for(&[(&relative, source)]);
        assert_eq!(
            uncovered_methods(&report),
            expected,
            "{extension}: {report:?}"
        );
    }
}

const FSHARP_CATALOG: &str = "[<Service>]\ntype CatalogService() =\n    member this.Total() : int = 2\n\n    member this.Load(name: string) : string =\n        let trimmed = name.Trim()\n        trimmed\n\n    member private this.Hidden() : int = 3\n";

#[test]
fn an_fsharp_class_and_utility_report_their_public_members() {
    let report = report_for(&[
        ("server/services/CatalogService.fs", FSHARP_CATALOG),
        (
            "server/utils/PriceUtils.fs",
            "type PriceUtils =\n    static member WithTax(cents: int64) : int64 =\n        cents + 1L\n\n    static member private Round(cents: int64) : int64 = cents\n",
        ),
    ]);
    assert_eq!(
        uncovered_methods(&report),
        ["Total", "Load", "WithTax"],
        "{report:?}"
    );
}

#[test]
fn an_fsharp_member_records_its_call_and_keeps_its_line_numbers() {
    let output = hooked("server/services/CatalogService.fs", FSHARP_CATALOG);
    let single = format!(
        "    member this.Total() : int = \n{0}BehaviorCalls.Record(\"CatalogService.Total\")\n# 3 \"server/services/CatalogService.fs\"\n{0}2\n",
        " ".repeat(32)
    );
    assert!(output.contains(&single), "{output}");
    assert!(
        output.contains("    member this.Load(name: string) : string =\n        BehaviorCalls.Record(\"CatalogService.Load\")\n# 6 \"server/services/CatalogService.fs\"\n        let trimmed = name.Trim()\n"),
        "{output}"
    );
    assert!(
        output.contains("    member private this.Hidden() : int = 3\n"),
        "{output}"
    );
}

#[test]
fn rust_snake_case_utility_coverage_names_the_actual_class() {
    let source = "pub struct TitleUtils;\nimpl TitleUtils { pub fn text() -> String { String::from(\"Ready\") } }\n";
    let class = RoleClass::read(
        Path::new("client/shared/utils/nested/title_utils.rs"),
        source,
    )
    .expect("coverage class")
    .expect("utility");
    assert_eq!(class.type_name, "TitleUtils");
    assert_eq!(class.key("text"), "TitleUtils.text");
}
