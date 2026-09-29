#![allow(clippy::expect_used)]

use std::path::Path;
use std::process::Command;

struct Units;

impl Units {
    const JS_SETUP: &str =
        "@TestSetup()\nclass StampTestSetup {\n  static configure() { return {}; }\n}\n";
    const JS_SERVICE: &str = "@Service\nexport default class StampService {\n  @Needs\n  clock;\n  @Needs\n  connectivity;\n\n  stamp() {\n    return `${this.clock.now()} ${this.connectivity.online()}`;\n  }\n}\n";
    const JS_UNIT: &str = "@Test(StampTestSetup)\nexport default class StampServiceTest {\n  @Fake\n  clock;\n  @Fake\n  connectivity;\n\n  async testUsesTheFakes() {\n    this.clock = { now: () => 1000 };\n    this.connectivity = { online: () => false };\n    const service = this.construct(StampService);\n    Assert.equals('1000 false', service.stamp());\n  }\n}\n";

    const GO_SETUP: &str = "// @TestSetup\ntype StampTestSetup struct{}\n\nfunc (StampTestSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {\n\treturn &BehaviorConfiguration{}, nil\n}\n";
    const GO_SERVICE: &str = "import \"strconv\"\n\n// @Service\ntype StampService struct {\n\t// @Needs\n\tclock Clock\n\t// @Needs\n\tconnectivity Connectivity\n}\n\nfunc (stamp *StampService) Stamp() string {\n\tnow, _ := stamp.clock.NowMs()\n\tonline, _ := stamp.connectivity.Online()\n\treturn strconv.FormatFloat(now, 'f', 0, 64) + \" \" + strconv.FormatBool(online)\n}\n";
    const GO_UNIT: &str = "import \"strings\"\n\n// @Test(StampTestSetup)\ntype StampServiceTest struct {\n\tbehaviors *BehaviorContext\n}\n\nfunc (test *StampServiceTest) uuid() error {\n\t_, err := SecureRandom{}.UUID()\n\treturn err\n}\n\nfunc (test *StampServiceTest) TestUsesTheBrowserFakes() {\n\tfakes := BrowserFakes{}\n\tfakes.Answer(\"clock\", \"now\", 1000)\n\tfakes.Answer(\"connectivity\", \"online\", false)\n\tservice := &StampService{}\n\tAssert.Equals(\"1000 false\", service.Stamp())\n}\n\nfunc (test *StampServiceTest) TestAsksForAnAnswer() {\n\terr := Assert.Throws(test.uuid)\n\tAssert.IsTrue(strings.Contains(err.Error(), \"BrowserFakes{}.Answer\"), \"a refusal that names BrowserFakes\")\n}\n";

    const RUBY_SETUP: &str =
        "#[TestSetup]\nclass StampTestSetup\n  def self.configure(execution)\n    {}\n  end\nend\n";
    const RUBY_SERVICE: &str = "#[Service]\nclass StampService\n  def stamp\n    \"#{Clock.new.now_ms.to_i} #{Connectivity.new.online}\"\n  end\nend\n";
    const RUBY_UNIT: &str = "#[Test(StampTestSetup)]\nclass StampServiceTest\n  def test_uses_the_browser_fakes\n    BrowserFakes.answer(\"clock\", \"now\", 1000)\n    BrowserFakes.answer(\"connectivity\", \"online\", false)\n    Assert.equals(\"1000 false\", StampService.new.stamp)\n  end\n\n  def test_asks_for_an_answer\n    error = Assert.throws { SecureRandom.new.uuid }\n    Assert.is_true(error.message.include?(\"BrowserFakes.answer\"), \"a refusal that names BrowserFakes.answer\")\n  end\nend\n";

    const SWIFT_SETUP: &str = "@TestSetup\nfinal class StampTestSetup {\n  static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters {\n    return BehaviorAdapters()\n  }\n}\n";
    const SWIFT_SERVICE: &str = "@Service\nfinal class StampService {\n  private let clock = Clock()\n  private let connectivity = Connectivity()\n  var online: Bool = false\n\n  func stamp() throws -> String {\n    \"\\(Int(clock.nowMs())) \\(try connectivity.online())\"\n  }\n\n  @OnEvent\n  func hearConnectivity(_ changed: ConnectivityChanged) {\n    online = changed.online\n  }\n}\n";
    const SWIFT_UNIT: &str = "@Test(StampTestSetup)\nfinal class StampServiceTest {\n  func testUsesTheBrowserFakes() async throws {\n    BrowserFakes.answer(\"clock\", \"now\", 1000)\n    BrowserFakes.answer(\"connectivity\", \"online\", false)\n    try Assert.equals(\"1000 false\", try StampService().stamp())\n  }\n\n  func testHearsTheBuiltInEventModel() async throws {\n    let service = StampService()\n    service.hearConnectivity(ConnectivityChanged(online: true))\n    try Assert.isTrue(service.online, \"the listener hears the event model\")\n  }\n\n  func testAsksForAnAnswer() async throws {\n    let error = try await Assert.throws { _ = try SecureRandom().uuid() }\n    try Assert.isTrue(String(describing: error).contains(\"BrowserFakes.answer\"), \"a refusal that names BrowserFakes.answer\")\n  }\n}\n";

    fn available(program: &str, flag: &str) -> bool {
        Command::new(program)
            .arg(flag)
            .output()
            .is_ok_and(|output| output.status.success())
    }

    fn write(root: &Path, relative: &str, source: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(path, source).expect("write");
    }

    fn passed(extension: &str, files: [&str; 3]) -> Vec<(String, bool)> {
        let root = tempfile::tempdir().expect("project");
        let [setup, service, unit] = files;
        Self::write(
            root.path(),
            &format!("client/tests/StampTestSetup.{extension}"),
            setup,
        );
        Self::write(
            root.path(),
            &format!("client/services/StampService.{extension}"),
            service,
        );
        let relative = format!("client/tests/StampServiceTest.{extension}");
        Self::write(root.path(), &relative, unit);
        let report = super::run(root.path(), Path::new(&relative)).expect("the unit runs");
        let mut results = Vec::new();
        for case in report.cases {
            let failure = case.error.unwrap_or_default();
            assert!(case.passed, "{}: {failure}", case.name);
            results.push((case.name, case.passed));
        }
        results
    }
}

#[test]
fn a_javascript_unit_fakes_the_browser_apis_of_a_service() {
    if !Units::available("node", "--version") {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let files = [Units::JS_SETUP, Units::JS_SERVICE, Units::JS_UNIT];
    let results = Units::passed("js", files);
    assert_eq!(results, [(String::from("testUsesTheFakes"), true)]);
}

#[test]
fn a_go_unit_answers_the_browser_apis_with_browser_fakes() {
    if !Units::available("go", "version") {
        eprintln!("skipping: go is unavailable");
        return;
    }
    let files = [Units::GO_SETUP, Units::GO_SERVICE, Units::GO_UNIT];
    let results = Units::passed("go", files);
    assert_eq!(results.len(), 2, "{results:?}");
}

#[test]
fn a_ruby_unit_answers_the_browser_apis_with_browser_fakes() {
    if !Units::available("ruby", "--version") {
        eprintln!("skipping: ruby is unavailable");
        return;
    }
    let files = [Units::RUBY_SETUP, Units::RUBY_SERVICE, Units::RUBY_UNIT];
    let results = Units::passed("rb", files);
    assert_eq!(results.len(), 2, "{results:?}");
}

#[test]
fn a_swift_unit_answers_the_browser_apis_with_browser_fakes() {
    if !cfg!(target_os = "macos") || !Units::available("swiftc", "--version") {
        eprintln!("skipping: swiftc on macOS is unavailable");
        return;
    }
    let files = [Units::SWIFT_SETUP, Units::SWIFT_SERVICE, Units::SWIFT_UNIT];
    let results = Units::passed("swift", files);
    assert_eq!(results.len(), 3, "{results:?}");
}
