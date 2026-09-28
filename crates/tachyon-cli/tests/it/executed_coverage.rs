#![allow(clippy::expect_used)]
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

struct Tested {
    passed: bool,
    report: String,
}

struct RunningServer {
    child: std::process::Child,
    first_line: std::sync::mpsc::Receiver<String>,
}

impl RunningServer {
    fn start(project: &Path, command: &str) -> Self {
        use std::io::BufRead as _;
        let mut child = CoverageProject::ty()
            .arg(command)
            .arg(project)
            .args(["--port", "0", "--no-self-restart"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("server");
        let output = child.stdout.take().expect("stdout");
        let errors = child.stderr.take().expect("stderr");
        let (sender, first_line) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut lines = std::io::BufReader::new(output).lines();
            let first = lines
                .next()
                .transpose()
                .expect("read stdout")
                .unwrap_or_default();
            let _ = sender.send(first);
            for line in lines {
                if line.is_err() {
                    break;
                }
            }
        });
        std::thread::spawn(move || {
            for line in std::io::BufReader::new(errors).lines() {
                if line.is_err() {
                    break;
                }
            }
        });
        Self { child, first_line }
    }

    fn ready(&self, expected: &str) {
        let line = self
            .first_line
            .recv_timeout(std::time::Duration::from_secs(30))
            .expect("first stdout line within deadline");
        assert!(line.starts_with(expected), "first stdout line: {line}");
    }
}

impl Drop for RunningServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

struct CoverageProject;

impl CoverageProject {
    const YON_FAKE_LANGUAGES: [(&'static str, &'static str); 3] =
        [("js", "bun"), ("ts", "bun"), ("rb", "ruby")];
    const PREVIEW_WAIT_SECONDS: u64 = 120;
    const TEST_ONLY_SERVICE_SETTINGS: &'static str = r#"{"visibility":{"check":false}}"#;

    fn ty() -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_ty"));
        command.env("YON_JAVASCRIPT_RUNTIME", "bun");
        command.env_remove("TAC_ALLOW_UNTESTED");
        command
    }

    fn available(program: &str) -> bool {
        Command::new(program)
            .arg(match program {
                "go" => "version",
                "kotlinc" => "-version",
                _ => "--version",
            })
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }

    fn fixture(extension: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tachyon-core/tests/fixtures/controller-test-units")
            .join(extension)
    }

    fn copy(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).expect("folder");
        for entry in std::fs::read_dir(from).expect("fixture folder").flatten() {
            let target = to.join(entry.file_name());
            if entry.path().is_dir() {
                Self::copy(&entry.path(), &target);
            } else {
                std::fs::copy(entry.path(), target).expect("fixture file");
            }
        }
    }

    fn write(root: &Path, relative: &str, source: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directories");
        std::fs::write(path, source).expect("source");
    }

    fn preview_files(root: &Path) {
        Self::write(
            root,
            "server/application.json",
            Self::TEST_ONLY_SERVICE_SETTINGS,
        );
        Self::write(root, "client/pages/tac.html", "<counter-heading />\n");
        Self::write(
            root,
            "client/components/counter-heading/tac.html",
            "<h1>Counter</h1>\n",
        );
    }

    fn preview(project: &Path) -> Tested {
        let mut child = Self::ty()
            .arg("preview")
            .arg(project)
            .args(["--port", "0", "--no-self-restart"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("ty preview");
        let output = Self::collect(child.stdout.take().expect("stdout"));
        let errors = Self::collect(child.stderr.take().expect("stderr"));
        let address = Self::ready_address(&output);
        std::thread::sleep(std::time::Duration::from_millis(500));
        let shown = address
            .as_deref()
            .map(Self::shown_diagnostics)
            .unwrap_or_default();
        let _ = child.kill();
        let _ = child.wait();
        Tested {
            passed: shown.trim() == "[]",
            report: format!(
                "{}{}{shown}",
                output.lock().expect("stdout text"),
                errors.lock().expect("stderr text")
            ),
        }
    }

    fn collect(
        stream: impl std::io::Read + Send + 'static,
    ) -> std::sync::Arc<std::sync::Mutex<String>> {
        use std::io::BufRead as _;
        let text = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let written = std::sync::Arc::clone(&text);
        std::thread::spawn(move || {
            for line in std::io::BufReader::new(stream)
                .lines()
                .map_while(Result::ok)
            {
                let mut text = written.lock().expect("collected text");
                text.push_str(&line);
                text.push('\n');
            }
        });
        text
    }

    fn ready_address(output: &std::sync::Mutex<String>) -> Option<String> {
        let deadline =
            std::time::Instant::now() + std::time::Duration::from_secs(Self::PREVIEW_WAIT_SECONDS);
        while std::time::Instant::now() < deadline {
            let text = output.lock().expect("stdout text").clone();
            if let Some(start) = text.find("ready at http://") {
                let rest = &text[start + "ready at http://".len()..];
                return rest.split('/').next().map(String::from);
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        None
    }

    fn shown_diagnostics(address: &str) -> String {
        use std::io::{Read as _, Write as _};
        let mut stream = std::net::TcpStream::connect(address).expect("preview connection");
        stream
            .write_all(
                b"GET /.tachyon/diagnostics HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
            )
            .expect("diagnostics request");
        let mut response = String::new();
        let _ = stream.read_to_string(&mut response);
        response
            .split_once("\r\n\r\n")
            .map(|(_, body)| body.to_owned())
            .unwrap_or(response)
    }

    fn test(project: &Path) -> Tested {
        let output: Output = Self::ty()
            .args(["test", "--coverage"])
            .arg(project)
            .output()
            .expect("ty test");
        Tested {
            passed: output.status.success(),
            report: format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ),
        }
    }
}

struct NamedMethod {
    realm: &'static str,
    extension: &'static str,
    program: &'static str,
    method: &'static str,
    case: &'static str,
    service: &'static str,
    setup: &'static str,
    unit: &'static str,
    named: &'static str,
    called: &'static str,
}

impl NamedMethod {
    const CHECK: &'static str = "CHECK";
    const CASES: [NamedMethod; 20] = [
        NamedMethod {
            realm: "server",
            extension: "js",
            program: "bun",
            method: "total",
            case: "testCounts",
            service: "@Service\nclass CounterService {\n  total() { return 2; }\n}\n",
            setup: "@TestSetup\nclass CounterTestSetup {\n  static configure() { return {}; }\n}\n",
            unit: "@Test(CounterTestSetup, {\"partition\":\"tests\"})\nexport default class CounterServiceTest {\n  async testCounts() {\n    if (CHECK) throw Error('failed');\n  }\n}\n",
            named: "'CounterService.total' === ''",
            called: "this.construct(CounterService).total() !== 2",
        },
        NamedMethod {
            realm: "server",
            extension: "ts",
            program: "bun",
            method: "total",
            case: "testCounts",
            service: "@Service\nclass CounterService {\n  total() { return 2; }\n}\n",
            setup: "@TestSetup\nclass CounterTestSetup {\n  static configure() { return {}; }\n}\n",
            unit: "@Test(CounterTestSetup, {\"partition\":\"tests\"})\nexport default class CounterServiceTest {\n  async testCounts() {\n    if (CHECK) throw Error('failed');\n  }\n}\n",
            named: "'CounterService.total' === ''",
            called: "this.construct(CounterService).total() !== 2",
        },
        NamedMethod {
            realm: "server",
            extension: "py",
            program: "python3",
            method: "total",
            case: "test_counts",
            service: "@service\nclass CounterService:\n    def total(self):\n        return 2\n",
            setup: "@TestSetup\nclass CounterTestSetup:\n    @staticmethod\n    def configure(execution):\n        return {}\n",
            unit: "@Test(CounterTestSetup, {\"partition\":\"tests\"})\nclass CounterServiceTest:\n    async def test_counts(self):\n        if CHECK:\n            raise Exception('failed')\n",
            named: "'CounterService.total' == ''",
            called: "CounterService().total() != 2",
        },
        NamedMethod {
            realm: "server",
            extension: "rb",
            program: "ruby",
            method: "total",
            case: "test_counts",
            service: "#[Service]\nclass CounterService\n  def total\n    2\n  end\nend\n",
            setup: "#[TestSetup]\nclass CounterTestSetup\n  def self.configure(execution)\n    {}\n  end\nend\n",
            unit: "#[Test(CounterTestSetup, {\"partition\":\"tests\"})]\nclass CounterServiceTest\n  def test_counts\n    raise 'failed' if CHECK\n  end\nend\n",
            named: "'CounterService.total' == ''",
            called: "construct(CounterService).total != 2",
        },
        NamedMethod {
            realm: "server",
            extension: "php",
            program: "php-cgi",
            method: "total",
            case: "testCounts",
            service: "<?php\n#[Service]\nclass CounterService {\n    public function total(): int { return 2; }\n}\n",
            setup: "<?php\n#[TestSetup]\nclass CounterTestSetup {\n    public static function configure($execution) { return []; }\n}\n",
            unit: "<?php\n#[Test(CounterTestSetup::class, {\"partition\":\"tests\"})]\nclass CounterServiceTest {\n    public function testCounts(): void {\n        if (CHECK) throw new Exception('failed');\n    }\n}\n",
            named: "'CounterService.total' === ''",
            called: "(new CounterService())->total() !== 2",
        },
        NamedMethod {
            realm: "server",
            extension: "java",
            program: "javac",
            method: "total",
            case: "testCounts",
            service: "@Service\nclass CounterService {\n    int total() {\n        return 2;\n    }\n}\n",
            setup: "@TestSetup\nclass CounterTestSetup {\n    static Behaviors.Configuration configure(Behaviors.Execution execution) {\n        return new Behaviors.Configuration(java.util.Map.of(), java.util.Map.of(), java.util.Map.of(), null, null);\n    }\n}\n",
            unit: "@Test(CounterTestSetup.class, {\"partition\":\"tests\"})\nclass CounterServiceTest {\n    void testCounts() {\n        if (CHECK) throw new IllegalStateException(\"failed\");\n    }\n}\n",
            named: "\"CounterService.total\".isEmpty()",
            called: "new CounterService().total() != 2",
        },
        NamedMethod {
            realm: "server",
            extension: "kt",
            program: "kotlinc",
            method: "total",
            case: "testCounts",
            service: "@Service\nclass CounterService {\n    fun total(): Int = 2\n}\n",
            setup: "@TestSetup\nclass CounterTestSetup {\n    companion object {\n        fun configure(execution: Behaviors.Execution): Behaviors.Configuration = Behaviors.Configuration()\n    }\n}\n",
            unit: "@Test(CounterTestSetup::class, {\"partition\":\"tests\"})\nclass CounterServiceTest {\n    fun testCounts() {\n        if (CHECK) throw IllegalStateException(\"failed\")\n    }\n}\n",
            named: "\"CounterService.total\".isEmpty()",
            called: "CounterService().total() != 2",
        },
        NamedMethod {
            realm: "server",
            extension: "cs",
            program: "dotnet",
            method: "Total",
            case: "testCounts",
            service: "[Service]\nclass CounterService {\n    public int Total() { return 2; }\n}\n",
            setup: "[TestSetup]\npublic sealed class CounterTestSetup {\n    public static BehaviorAdapters configure(BehaviorExecution execution) { return new BehaviorAdapters(); }\n}\n",
            unit: "[Test(typeof(CounterTestSetup), {\"partition\":\"tests\"})]\nsealed class CounterServiceTest {\n    void testCounts() {\n        if (CHECK) throw new System.Exception(\"failed\");\n    }\n}\n",
            named: "\"CounterService.Total\" == \"\"",
            called: "new CounterService().Total() != 2",
        },
        NamedMethod {
            realm: "server",
            extension: "rs",
            program: "cargo",
            method: "total",
            case: "testCounts",
            service: "#[Service]\nstruct CounterService {}\nimpl CounterService {\n    fn new() -> Self { Self {} }\n    fn total(&self) -> i32 { 2 }\n}\n",
            setup: "#[TestSetup]\nstruct CounterTestSetup {}\nimpl CounterTestSetup {\n    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {\n        Ok(BehaviorConfiguration::default())\n    }\n}\n",
            unit: "#[Test(CounterTestSetup, {\"partition\":\"tests\"})]\nstruct CounterServiceTest { behaviors: std::cell::RefCell<Option<BehaviorContext>> }\nimpl CounterServiceTest {\n    fn new() -> Self { Self { behaviors: std::cell::RefCell::new(None) } }\n    fn testCounts(&self) -> BehaviorResult<BehaviorValue> {\n        if CHECK {\n            return Err(BehaviorError { code: \"adapter\", message: \"failed\", path: None });\n        }\n        Ok(BehaviorValue::Null)\n    }\n}\n",
            named: "\"CounterService.total\".is_empty()",
            called: "CounterService::new().total() != 2",
        },
        NamedMethod {
            realm: "server",
            extension: "go",
            program: "go",
            method: "Total",
            case: "TestCounts",
            service: "// @Service\ntype CounterService struct{}\n\nfunc (service *CounterService) Total() int {\n\treturn 2\n}\n",
            setup: "// @TestSetup\ntype CounterTestSetup struct{}\n\nfunc (CounterTestSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {\n\treturn NewBehaviorConfiguration(), nil\n}\n",
            unit: "// @Test(CounterTestSetup, {\"partition\":\"tests\"})\ntype CounterServiceTest struct {\n\tbehaviors *BehaviorContext\n}\n\nfunc (test *CounterServiceTest) TestCounts() error {\n\tif CHECK {\n\t\treturn &BehaviorError{Code: \"adapter\", Message: \"failed\"}\n\t}\n\treturn nil\n}\n",
            named: "len(\"CounterService.Total\") == 0",
            called: "(&CounterService{}).Total() != 2",
        },
        NamedMethod {
            realm: "server",
            extension: "dart",
            program: "dart",
            method: "total",
            case: "testCounts",
            service: "@Service\nclass CounterService {\n  int total() => 2;\n}\n",
            setup: "@TestSetup\nclass CounterTestSetup {\n  static BehaviorAdapters configure(BehaviorExecution execution) {\n    return BehaviorAdapters();\n  }\n}\n",
            unit: "@Test(CounterTestSetup, {\"partition\":\"tests\"})\nclass CounterServiceTest {\n  Future<void> testCounts() async {\n    if (CHECK) throw StateError('failed');\n  }\n}\n",
            named: "'CounterService.total'.isEmpty",
            called: "CounterService().total() != 2",
        },
        NamedMethod {
            realm: "client",
            extension: "js",
            program: "bun",
            method: "total",
            case: "testCounts",
            service: "@Service\nexport default class CounterService {\n  total() {\n    return 2\n  }\n}\n",
            setup: "@TestSetup\nclass CounterTestSetup {\n  static configure() {\n    return {}\n  }\n}\n",
            unit: "@Test(CounterTestSetup, {\"partition\":\"tests\"})\nexport default class CounterServiceTest {\n  async testCounts() {\n    if (CHECK) throw Error('failed')\n  }\n}\n",
            named: "'CounterService.total' === ''",
            called: "new CounterService().total() !== 2",
        },
        NamedMethod {
            realm: "client",
            extension: "ts",
            program: "bun",
            method: "total",
            case: "testCounts",
            service: "@Service\nexport default class CounterService {\n  total(): number {\n    return 2\n  }\n}\n",
            setup: "@TestSetup\nclass CounterTestSetup {\n  static configure() {\n    return {}\n  }\n}\n",
            unit: "@Test(CounterTestSetup, {\"partition\":\"tests\"})\nexport default class CounterServiceTest {\n  async testCounts() {\n    if (CHECK) throw Error('failed')\n  }\n}\n",
            named: "'CounterService.total' === ''",
            called: "new CounterService().total() !== 2",
        },
        NamedMethod {
            realm: "client",
            extension: "rb",
            program: "ruby",
            method: "total",
            case: "test_counts",
            service: "#[Service]\nclass CounterService\n  def total\n    2\n  end\nend\n",
            setup: "#[TestSetup]\nclass CounterTestSetup\n  def self.configure(execution)\n    {}\n  end\nend\n",
            unit: "#[Test(CounterTestSetup, {\"partition\":\"tests\"})]\nclass CounterServiceTest\n  def test_counts\n    raise 'failed' if CHECK\n  end\nend\n",
            named: "'CounterService.total' == ''",
            called: "CounterService.new.total != 2",
        },
        NamedMethod {
            realm: "client",
            extension: "dart",
            program: "dart",
            method: "total",
            case: "testCounts",
            service: "@Service\nclass CounterService {\n  int total() => 2;\n}\n",
            setup: "@TestSetup\nclass CounterTestSetup {\n  static BehaviorAdapters configure(BehaviorExecution execution) {\n    return BehaviorAdapters();\n  }\n}\n",
            unit: "@Test(CounterTestSetup, {\"partition\":\"tests\"})\nclass CounterServiceTest {\n  Future<void> testCounts() async {\n    if (CHECK) throw StateError('failed');\n  }\n}\n",
            named: "'CounterService.total'.isEmpty",
            called: "CounterService().total() != 2",
        },
        NamedMethod {
            realm: "client",
            extension: "rs",
            program: "cargo",
            method: "total",
            case: "testCounts",
            service: "#[Service]\n#[derive(Default)]\nstruct CounterService {}\n\nimpl CounterService {\n    fn total(&self) -> i32 {\n        2\n    }\n}\n",
            setup: "#[TestSetup]\nstruct CounterTestSetup {}\nimpl CounterTestSetup {\n    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {\n        Ok(BehaviorConfiguration::default())\n    }\n}\n",
            unit: "#[Test(CounterTestSetup, {\"partition\":\"tests\"})]\nstruct CounterServiceTest {\n    behaviors: std::cell::RefCell<Option<BehaviorContext>>,\n}\nimpl CounterServiceTest {\n    fn new() -> Self {\n        Self { behaviors: std::cell::RefCell::new(None) }\n    }\n\n    fn testCounts(&self) -> BehaviorResult<BehaviorValue> {\n        if CHECK {\n            return Err(BehaviorError { code: \"assertion\", message: \"failed\", path: None });\n        }\n        Ok(BehaviorValue::Null)\n    }\n}\n",
            named: "\"CounterService.total\".is_empty()",
            called: "CounterService::default().total() != 2",
        },
        NamedMethod {
            realm: "client",
            extension: "cs",
            program: "dotnet",
            method: "Total",
            case: "testCounts",
            service: "[Service]\npublic sealed class CounterService {\n    public int Total() { return 2; }\n}\n",
            setup: "[TestSetup]\npublic sealed class CounterTestSetup {\n    public static BehaviorAdapters configure(BehaviorExecution execution) { return new BehaviorAdapters(); }\n}\n",
            unit: "[Test(typeof(CounterTestSetup), {\"partition\":\"tests\"})]\nsealed class CounterServiceTest {\n    void testCounts() {\n        if (CHECK) throw new System.Exception(\"failed\");\n    }\n}\n",
            named: "\"CounterService.Total\" == \"\"",
            called: "new CounterService().Total() != 2",
        },
        NamedMethod {
            realm: "client",
            extension: "go",
            program: "go",
            method: "Total",
            case: "TestCounts",
            service: "// @Service\ntype CounterService struct{}\n\nfunc (service *CounterService) Total() int {\n\treturn 2\n}\n",
            setup: "// @TestSetup\ntype CounterTestSetup struct{}\n\nfunc (CounterTestSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {\n\treturn &BehaviorConfiguration{}, nil\n}\n",
            unit: "// @Test(CounterTestSetup, {\"partition\":\"tests\"})\ntype CounterServiceTest struct {\n\tbehaviors *BehaviorContext\n}\n\nfunc (test *CounterServiceTest) TestCounts() error {\n\tif CHECK {\n\t\treturn &BehaviorError{Code: \"assertion\", Message: \"failed\"}\n\t}\n\treturn nil\n}\n",
            named: "len(\"CounterService.Total\") == 0",
            called: "(&CounterService{}).Total() != 2",
        },
        NamedMethod {
            realm: "client",
            extension: "swift",
            program: "swiftc",
            method: "total",
            case: "testCounts",
            service: "@Service\nfinal class CounterService {\n    func total() -> Int { 2 }\n}\n",
            setup: "@TestSetup\nfinal class CounterTestSetup {\n    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters {\n        BehaviorAdapters()\n    }\n}\n",
            unit: "@Test(CounterTestSetup, {\"partition\":\"tests\"})\nfinal class CounterServiceTest {\n    func testCounts() async throws {\n        if CHECK { throw BehaviorError(\"FAILED\") }\n    }\n}\n",
            named: "\"CounterService.total\".isEmpty",
            called: "CounterService().total() != 2",
        },
        NamedMethod {
            realm: "client",
            extension: "kt",
            program: "kotlinc",
            method: "total",
            case: "testCounts",
            service: "@Service\nclass CounterService {\n    fun total(): Int = 2\n}\n",
            setup: "@TestSetup\nclass CounterTestSetup {\n    companion object {\n        fun configure(execution: Behaviors.Execution): Behaviors.Configuration = Behaviors.Configuration()\n    }\n}\n",
            unit: "@Test(CounterTestSetup::class, {\"partition\":\"tests\"})\nclass CounterServiceTest {\n    fun testCounts() {\n        if (CHECK) throw IllegalStateException(\"failed\")\n    }\n}\n",
            named: "\"CounterService.total\".isEmpty()",
            called: "CounterService().total() != 2",
        },
    ];

    fn project(&self, root: &Path, check: &str) {
        let extension = self.extension;
        let realm = self.realm;
        CoverageProject::write(
            root,
            &format!("{realm}/services/CounterService.{extension}"),
            self.service,
        );
        CoverageProject::write(
            root,
            &format!("{realm}/tests/CounterTestSetup.{extension}"),
            self.setup,
        );
        CoverageProject::write(
            root,
            &format!("{realm}/tests/CounterServiceTest.{extension}"),
            &self.unit.replace(Self::CHECK, check),
        );
    }

    fn label(&self) -> String {
        format!("{} {}", self.realm, self.extension)
    }
}

#[test]
fn a_method_a_case_only_names_fails_coverage_and_a_called_method_passes() {
    for case in &NamedMethod::CASES {
        if !CoverageProject::available(case.program) {
            eprintln!("skipping {}: {} is unavailable", case.label(), case.program);
            continue;
        }
        let root = tempfile::tempdir().expect("project");
        case.project(root.path(), case.named);
        let named = CoverageProject::test(root.path());
        let missing = format!("missing  {}", case.method);
        assert!(!named.passed, "{}: {}", case.label(), named.report);
        assert!(
            named.report.contains(&missing),
            "{}: {}",
            case.label(),
            named.report
        );
        assert!(
            named.report.contains("TY2022"),
            "{}: {}",
            case.label(),
            named.report
        );
        case.project(root.path(), case.called);
        let called = CoverageProject::test(root.path());
        let first = format!(
            "called   {} (first by CounterServiceTest.{})",
            case.method, case.case
        );
        assert!(called.passed, "{}: {}", case.label(), called.report);
        assert!(
            called.report.contains(&first),
            "{}: {}",
            case.label(),
            called.report
        );
    }
}

#[test]
fn preview_shows_a_method_no_case_called_after_running_the_tests_itself() {
    let case = &NamedMethod::CASES[0];
    if !CoverageProject::available(case.program) {
        eprintln!(
            "skipping the preview check: {} is unavailable",
            case.program
        );
        return;
    }
    let root = tempfile::tempdir().expect("project");
    case.project(root.path(), case.named);
    CoverageProject::preview_files(root.path());
    let refused = CoverageProject::preview(root.path());
    assert!(!refused.passed, "{}", refused.report);
    assert!(root.path().join(".tachyon/coverage/calls.json").is_file());
    assert!(
        refused
            .report
            .contains("no test case called CounterService.total"),
        "{}",
        refused.report
    );
    assert!(refused.report.contains("TY2023"), "{}", refused.report);
}

#[test]
fn serve_starts_without_running_tests_or_writing_a_coverage_record() {
    let case = &NamedMethod::CASES[0];
    let root = tempfile::tempdir().expect("project");
    case.project(root.path(), case.named);
    CoverageProject::preview_files(root.path());
    let server = RunningServer::start(root.path(), "serve");
    server.ready("Tachyon server ready at http://127.0.0.1:");
    assert!(!root.path().join(".tachyon/coverage/calls.json").exists());
}

#[test]
fn preview_records_test_calls_before_announcing_readiness_as_its_first_line() {
    let case = &NamedMethod::CASES[0];
    let root = tempfile::tempdir().expect("project");
    case.project(root.path(), case.called);
    CoverageProject::preview_files(root.path());
    let server = RunningServer::start(root.path(), "preview");
    server.ready("Tachyon preview ready at http://127.0.0.1:");
    let record = tachyon_core::method_coverage::RecordedCalls::read(root.path()).expect("record");
    assert_eq!(
        record.first_case("server", "CounterService.total"),
        Some("CounterServiceTest.testCounts")
    );
}

#[test]
fn preview_reuses_a_current_call_record_without_rewriting_it() {
    let case = &NamedMethod::CASES[0];
    let root = tempfile::tempdir().expect("project");
    case.project(root.path(), case.called);
    CoverageProject::preview_files(root.path());
    let first = RunningServer::start(root.path(), "preview");
    first.ready("Tachyon preview ready at http://127.0.0.1:");
    drop(first);
    let record = root.path().join(".tachyon/coverage/calls.json");
    let original = std::fs::metadata(&record)
        .expect("record")
        .modified()
        .expect("record time");
    let second = RunningServer::start(root.path(), "preview");
    second.ready("Tachyon preview ready at http://127.0.0.1:");
    assert_eq!(
        std::fs::metadata(record)
            .expect("record")
            .modified()
            .expect("record time"),
        original
    );
}

#[test]
fn preview_rechecks_stale_calls_and_shows_newly_uncovered_methods() {
    let case = &NamedMethod::CASES[0];
    let root = tempfile::tempdir().expect("project");
    case.project(root.path(), case.called);
    CoverageProject::preview_files(root.path());
    let first = RunningServer::start(root.path(), "preview");
    first.ready("Tachyon preview ready at http://127.0.0.1:");
    drop(first);
    case.project(root.path(), case.named);
    let record = root.path().join(".tachyon/coverage/calls.json");
    std::fs::File::open(record)
        .expect("record")
        .set_times(std::fs::FileTimes::new().set_modified(std::time::UNIX_EPOCH))
        .expect("old record time");
    let refused = CoverageProject::preview(root.path());
    assert!(!refused.passed, "{}", refused.report);
    assert!(refused.report.contains("TY2023"), "{}", refused.report);
    assert!(
        refused.report.contains("Tachyon preview ready"),
        "{}",
        refused.report
    );
    let calls = tachyon_core::method_coverage::RecordedCalls::read(root.path()).expect("record");
    assert_eq!(calls.first_case("server", "CounterService.total"), None);
}

#[test]
fn preview_shows_failed_tests_even_when_they_called_every_method() {
    let case = &NamedMethod::CASES[0];
    let root = tempfile::tempdir().expect("project");
    case.project(root.path(), "this.construct(CounterService).total() === 2");
    CoverageProject::preview_files(root.path());
    let refused = CoverageProject::preview(root.path());
    assert!(!refused.passed, "{}", refused.report);
    assert!(refused.report.contains("TY2020"), "{}", refused.report);
    assert!(
        refused
            .report
            .contains("FAIL CounterServiceTest.testCounts"),
        "{}",
        refused.report
    );
    assert!(
        refused.report.contains("Tachyon preview ready"),
        "{}",
        refused.report
    );
    assert!(tachyon_core::method_coverage::RecordedCalls::read_current(root.path()).is_none());
}

#[test]
fn preview_does_not_accept_a_fresh_record_from_a_failed_explicit_test_run() {
    let case = &NamedMethod::CASES[0];
    let root = tempfile::tempdir().expect("project");
    case.project(root.path(), "this.construct(CounterService).total() === 2");
    CoverageProject::preview_files(root.path());
    let tested = CoverageProject::test(root.path());
    assert!(!tested.passed, "{}", tested.report);
    assert!(tested.report.contains("TY2020"), "{}", tested.report);
    let record = tachyon_core::method_coverage::RecordedCalls::read(root.path()).expect("record");
    assert_eq!(
        record.first_case("server", "CounterService.total"),
        Some("CounterServiceTest.testCounts")
    );
    assert!(tachyon_core::method_coverage::RecordedCalls::read_current(root.path()).is_none());
    let refused = CoverageProject::preview(root.path());
    assert!(!refused.passed, "{}", refused.report);
    assert!(refused.report.contains("TY2020"), "{}", refused.report);
    assert!(
        refused.report.contains("Tachyon preview ready"),
        "{}",
        refused.report
    );
}

#[test]
fn a_call_on_a_fake_does_not_cover_the_real_service() {
    for (extension, program) in CoverageProject::YON_FAKE_LANGUAGES {
        if !CoverageProject::available(program) {
            eprintln!("skipping {extension}: {program} is unavailable");
            continue;
        }
        let root = tempfile::tempdir().expect("project");
        CoverageProject::copy(&CoverageProject::fixture(extension), root.path());
        let service_unit = root
            .path()
            .join(format!("server/tests/GreetingServiceTest.{extension}"));
        let service_test = std::fs::read_to_string(&service_unit).expect("service unit");
        std::fs::remove_file(&service_unit).expect("remove the service unit");
        let faked = CoverageProject::test(root.path());
        assert!(!faked.passed, "{extension}: {}", faked.report);
        assert!(
            faked.report.contains("missing  message"),
            "{extension}: {}",
            faked.report
        );
        std::fs::write(&service_unit, service_test).expect("restore the service unit");
        let called = CoverageProject::test(root.path());
        assert!(called.passed, "{extension}: {}", called.report);
        assert!(
            called
                .report
                .contains("called   message (first by GreetingServiceTest."),
            "{extension}: {}",
            called.report
        );
    }
}
