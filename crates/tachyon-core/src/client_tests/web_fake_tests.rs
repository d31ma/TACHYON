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

    const CSHARP_SETUP: &str = "[TestSetup]\npublic sealed class StampTestSetup {\n    public static BehaviorAdapters configure(BehaviorExecution execution) { return new BehaviorAdapters(); }\n}\n";
    const CSHARP_SERVICE: &str = "[Service]\npublic class StampService\n{\n    private readonly Clock clock = new();\n    private readonly Connectivity connectivity = new();\n    public bool Online = false;\n\n    public string Stamp() => clock.NowMs() + \" \" + (connectivity.Online() ? \"online\" : \"offline\");\n\n    [OnEvent]\n    public void Hear(ConnectivityChanged changed)\n    {\n        Online = changed.Online;\n    }\n}\n";
    const CSHARP_UNIT: &str = "[Test(typeof(StampTestSetup), {\"partition\":\"tests\"})]\nsealed class StampServiceTest {\n    void testUsesTheBrowserFakes() {\n        BrowserFakes.Answer(\"clock\", \"now\", 1000);\n        BrowserFakes.Answer(\"connectivity\", \"online\", false);\n        Assert.equals(\"1000 offline\", new StampService().Stamp());\n    }\n    void testHearsTheBuiltInEventModel() {\n        var service = new StampService();\n        service.Hear(new ConnectivityChanged(true));\n        Assert.isTrue(service.Online, \"the listener hears the event model\");\n    }\n    void testAsksForAnAnswer() {\n        var error = Assert.throws(() => new SecureRandom().Uuid());\n        Assert.isTrue(error.Message.Contains(\"BrowserFakes.Answer\"), \"a refusal that names BrowserFakes.Answer\");\n    }\n}\n";

    const FSHARP_SETUP: &str = "[<TestSetup>]\ntype StampTestSetup() =\n    static member configure(execution: BehaviorExecution) : BehaviorAdapters =\n        BehaviorAdapters()\n";
    const FSHARP_SERVICE: &str = "[<Service>]\ntype StampService() =\n    let clock = Clock()\n    let connectivity = Connectivity()\n    member val Online : bool = false with get, set\n\n    member this.Stamp() : string =\n        let state = if connectivity.Online() then \"online\" else \"offline\"\n        string (clock.NowMs()) + \" \" + state\n\n    [<OnEvent>]\n    member this.Hear(changed: ConnectivityChanged) : unit =\n        this.Online <- changed.Online\n";
    const FSHARP_UNIT: &str = "[<Test(typeof<StampTestSetup>, {\"partition\":\"tests\"})>]\ntype StampServiceTest() =\n    member this.testUsesTheBrowserFakes() : unit =\n        BrowserFakes.Answer(\"clock\", \"now\", 1000)\n        BrowserFakes.Answer(\"connectivity\", \"online\", false)\n        Assert.equals(\"1000 offline\", StampService().Stamp())\n\n    member this.testHearsTheBuiltInEventModel() : unit =\n        let service = StampService()\n        service.Hear({ Online = true })\n        Assert.isTrue(service.Online, \"the listener hears the event model\")\n\n    member this.testAsksForAnAnswer() : unit =\n        let error = Assert.throws(fun () -> SecureRandom().Uuid() |> ignore)\n        Assert.isTrue(error.Message.Contains(\"BrowserFakes.Answer\"), \"a refusal that names BrowserFakes.Answer\")\n";

    const JS_WINDOW_SERVICE: &str = "@Service\nexport default class StampService {\n  @Needs\n  windowControls;\n\n  shrink() {\n    this.windowControls.minimize();\n    return this.windowControls.granted('minimize');\n  }\n}\n";
    const JS_WINDOW_UNIT: &str = "@Test(StampTestSetup)\nexport default class StampServiceTest {\n  @Fake\n  windowControls;\n\n  async testUsesTheFakeWindow() {\n    this.windowControls = { minimize: () => null, granted: () => true };\n    const service = this.construct(StampService);\n    Assert.isTrue(service.shrink(), 'the fake window answers');\n  }\n}\n";
    const GO_WINDOW_SERVICE: &str = "// @Service\ntype StampService struct {\n\t// @Needs\n\twindowControls WindowControls\n}\n\nfunc (stamp *StampService) Allowed() bool {\n\tgranted, _ := stamp.windowControls.Granted(\"minimize\")\n\treturn granted\n}\n";
    const GO_WINDOW_UNIT: &str = "// @Test(StampTestSetup)\ntype StampServiceTest struct {\n\tbehaviors *BehaviorContext\n}\n\nfunc (test *StampServiceTest) TestUsesTheFakeWindow() {\n\tBrowserFakes{}.Answer(\"windowControls\", \"granted\", true)\n\tservice := &StampService{}\n\tAssert.IsTrue(service.Allowed(), \"the fake window answers\")\n}\n";
    const RUBY_WINDOW_SERVICE: &str = "#[Service]\nclass StampService\n  def allowed\n    WindowControls.new.granted(\"minimize\")\n  end\nend\n";
    const RUBY_WINDOW_UNIT: &str = "#[Test(StampTestSetup)]\nclass StampServiceTest\n  def test_uses_the_fake_window\n    BrowserFakes.answer(\"windowControls\", \"granted\", true)\n    Assert.is_true(StampService.new.allowed, \"the fake window answers\")\n  end\nend\n";
    const SWIFT_WINDOW_SERVICE: &str = "@Service\nfinal class StampService {\n  private let windowControls = WindowControls()\n\n  func allowed() throws -> Bool {\n    return try windowControls.granted(\"minimize\")\n  }\n}\n";
    const SWIFT_WINDOW_UNIT: &str = "@Test(StampTestSetup)\nfinal class StampServiceTest {\n  func testUsesTheFakeWindow() async throws {\n    BrowserFakes.answer(\"windowControls\", \"granted\", true)\n    try Assert.isTrue(try StampService().allowed(), \"the fake window answers\")\n  }\n}\n";

    const CSHARP_WINDOW_SERVICE: &str = "[Service]\npublic class StampService\n{\n    private readonly WindowControls windowControls = new();\n\n    public bool Allowed() => windowControls.Granted(\"minimize\");\n}\n";
    const CSHARP_WINDOW_UNIT: &str = "[Test(typeof(StampTestSetup), {\"partition\":\"tests\"})]\nsealed class StampServiceTest {\n    void testUsesTheFakeWindow() {\n        BrowserFakes.Answer(\"windowControls\", \"granted\", true);\n        Assert.isTrue(new StampService().Allowed(), \"the fake window answers\");\n    }\n}\n";
    const FSHARP_WINDOW_SERVICE: &str = "[<Service>]\ntype StampService() =\n    let windowControls = WindowControls()\n\n    member this.Allowed() : bool = windowControls.Granted(\"minimize\")\n";
    const FSHARP_WINDOW_UNIT: &str = "[<Test(typeof<StampTestSetup>, {\"partition\":\"tests\"})>]\ntype StampServiceTest() =\n    member this.testUsesTheFakeWindow() : unit =\n        BrowserFakes.Answer(\"windowControls\", \"granted\", true)\n        Assert.isTrue(StampService().Allowed(), \"the fake window answers\")\n";

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

#[test]
fn a_csharp_unit_answers_the_browser_apis_with_browser_fakes() {
    if !Units::available("dotnet", "--version") {
        eprintln!("skipping: dotnet is unavailable");
        return;
    }
    let files = [
        Units::CSHARP_SETUP,
        Units::CSHARP_SERVICE,
        Units::CSHARP_UNIT,
    ];
    let results = Units::passed("cs", files);
    assert_eq!(results.len(), 3, "{results:?}");
}

#[test]
fn an_fsharp_unit_answers_the_browser_apis_with_browser_fakes() {
    if !Units::available("dotnet", "--version") {
        eprintln!("skipping: dotnet is unavailable");
        return;
    }
    let files = [
        Units::FSHARP_SETUP,
        Units::FSHARP_SERVICE,
        Units::FSHARP_UNIT,
    ];
    let results = Units::passed("fs", files);
    assert_eq!(results.len(), 3, "{results:?}");
}

#[test]
fn a_unit_fakes_the_window_controls_in_every_language_with_browser_fakes() {
    let cases = [
        (
            "node",
            "--version",
            "js",
            [
                Units::JS_SETUP,
                Units::JS_WINDOW_SERVICE,
                Units::JS_WINDOW_UNIT,
            ],
        ),
        (
            "go",
            "version",
            "go",
            [
                Units::GO_SETUP,
                Units::GO_WINDOW_SERVICE,
                Units::GO_WINDOW_UNIT,
            ],
        ),
        (
            "ruby",
            "--version",
            "rb",
            [
                Units::RUBY_SETUP,
                Units::RUBY_WINDOW_SERVICE,
                Units::RUBY_WINDOW_UNIT,
            ],
        ),
        (
            "swiftc",
            "--version",
            "swift",
            [
                Units::SWIFT_SETUP,
                Units::SWIFT_WINDOW_SERVICE,
                Units::SWIFT_WINDOW_UNIT,
            ],
        ),
        (
            "dotnet",
            "--version",
            "cs",
            [
                Units::CSHARP_SETUP,
                Units::CSHARP_WINDOW_SERVICE,
                Units::CSHARP_WINDOW_UNIT,
            ],
        ),
        (
            "dotnet",
            "--version",
            "fs",
            [
                Units::FSHARP_SETUP,
                Units::FSHARP_WINDOW_SERVICE,
                Units::FSHARP_WINDOW_UNIT,
            ],
        ),
    ];
    for (program, flag, extension, files) in cases {
        if !Units::available(program, flag) {
            eprintln!("skipping {extension}: {program} is unavailable");
            continue;
        }
        let results = Units::passed(extension, files);
        assert_eq!(results.len(), 1, "{extension}: {results:?}");
    }
}
