use super::{
    WebApis, WebLanguage, dotnet_wasm_ready, lua_unavailable, publish_java, publish_swift,
};
use std::path::Path;
use std::process::Command;

struct CodeScannerTests;

impl CodeScannerTests {
    const MODULE: &str = include_str!("../template/tac-code-scanner.js");

    const SERVICE_DRIVER: &str = r";(0, eval)(`${await readFile('./tac-web.js', 'utf8')}\nglobalThis.TacWebApis = TacWebApis`)
const { default: TacApplicationEvents } = await import('./application-events.mjs')
const { default: StampService, __tachyon_event_plan__: plan } = await import('./StampService.js')
__GRANT_STUBS__
grants.content = 'camera'
globalThis.isSecureContext = true
const camera = { answer: 'stream', stopped: 0 }
const element = (tag) => ({
  tag, children: [], attributes: {}, listeners: {}, readyState: 4, videoWidth: 640, videoHeight: 480,
  setAttribute(name, value) { this.attributes[name] = value },
  append(...items) { this.children.push(...items) },
  remove() { document.body.children = document.body.children.filter((child) => child !== this) },
  attachShadow() { this.shadow = element('#shadow'); return this.shadow },
  addEventListener(type, listener) { (this.listeners[type] ??= []).push(listener) },
  querySelector(name) { for (const child of this.children) { if (child.tag === name) return child; const found = child.querySelector(name); if (found) return found } return null },
  showModal() {}, close() {}, focus() {}, play: async () => {},
  getContext: () => ({ drawImage: () => {} }),
})
Object.assign(globalThis.document, { body: element('body'), createElement: element, removeEventListener: () => {} })
navigator.mediaDevices = {
  getUserMedia: async () => {
    if (camera.answer !== 'stream') throw new DOMException('no', camera.answer)
    return { getTracks: () => [{ stop: () => { camera.stopped += 1 } }] }
  },
  enumerateDevices: async () => [{ kind: 'videoinput' }],
}
globalThis.BarcodeDetector = class {
  static getSupportedFormats = async () => ['qr_code']
  detect = async () => [{ rawValue: 'tachyon-link:demo' }]
}
await import('./tac-code-scanner.mjs')
const settle = (milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds))
const bus = new TacApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })
TacWebApis.events = { publish: (name, value) => bus.publish(name, value), send: (owner, name, value) => bus.send(owner, name, value) }
const link = new StampService({})
bus.attach(link, plan.types[0], { applicationEventPublisher: null })
await bus.initialize(link)
await settle(20)
const ready = link.available()
link.link()
await settle(400)
camera.answer = 'NotAllowedError'
link.link()
await settle(50)
const denied = link.failure
camera.answer = 'stream'
link.link()
await settle(20)
link.stop()
await settle(50)
console.log(JSON.stringify({ ready, linked: link.linked, denied, failure: link.failure, stopped: camera.stopped }))
process.exit(0)
";

    const RUST: &str = r"#[Service]
#[derive(Default)]
struct StampService {
    code_scanner: CodeScanner,
    linked: String,
    failure: String,
}

impl StampService {
    pub fn available(&self) -> bool {
        self.code_scanner.supported().unwrap_or(false)
    }

    pub fn link(&self) {
        let _ = self.code_scanner.scan();
    }

    pub fn stop(&self) {
        let _ = self.code_scanner.cancel();
    }

    #[OnEvent]
    pub fn hear_scanned(&mut self, scanned: CodeScanned) {
        self.linked = scanned.text;
    }

    #[OnEvent]
    pub fn hear_failed(&mut self, failed: ScanFailed) {
        self.failure = failed.reason;
    }
}
";

    const GO: &str = r"// @Service
type StampService struct {
	// @Needs
	codeScanner CodeScanner
	Linked      string
	Failure     string
}

func (link *StampService) Available() bool {
	supported, _ := link.codeScanner.Supported()
	return supported
}

func (link *StampService) Link() {
	_ = link.codeScanner.Scan()
}

func (link *StampService) Stop() {
	_ = link.codeScanner.Cancel()
}

// @OnEvent
func (link *StampService) HearScanned(scanned CodeScanned) {
	link.Linked = scanned.Text
}

// @OnEvent
func (link *StampService) HearFailed(failed ScanFailed) {
	link.Failure = failed.Reason
}
";

    const CSHARP: &str = r#"[Service]
public class StampService
{
    private readonly CodeScanner codeScanner = new();
    public string Linked = "";
    public string Failure = "";

    public bool Available() => codeScanner.Supported();

    public void Link() => codeScanner.Scan();

    public void Stop() => codeScanner.Cancel();

    [OnEvent]
    public void HearScanned(CodeScanned scanned)
    {
        Linked = scanned.Text;
    }

    [OnEvent]
    public void HearFailed(ScanFailed failed)
    {
        Failure = failed.Reason;
    }
}
"#;

    const FSHARP: &str = r#"[<Service>]
type StampService() =
    let codeScanner = CodeScanner()
    member val Linked: string = "" with get, set
    member val Failure: string = "" with get, set
    member this.Available() : bool = codeScanner.Supported()
    member this.Link() : unit = codeScanner.Scan()
    member this.Stop() : unit = codeScanner.Cancel()
    [<OnEvent>]
    member this.HearScanned(scanned: CodeScanned) : unit = this.Linked <- scanned.Text
    [<OnEvent>]
    member this.HearFailed(failed: ScanFailed) : unit = this.Failure <- failed.Reason
"#;

    const SWIFT: &str = r#"@Service
final class StampService {
    private let codeScanner = CodeScanner()
    var linked: String = ""
    var failure: String = ""

    func available() -> Bool {
        (try? codeScanner.supported()) ?? false
    }

    func link() throws {
        try codeScanner.scan()
    }

    func stop() throws {
        try codeScanner.cancel()
    }

    @OnEvent
    func hearScanned(_ scanned: CodeScanned) {
        linked = scanned.text
    }

    @OnEvent
    func hearFailed(_ failed: ScanFailed) {
        failure = failed.reason
    }
}
"#;

    const KOTLIN: &str = r#"@Service
class StampService {
    private val codeScanner = CodeScanner()
    var linked: String = ""
    var failure: String = ""

    fun available(): Boolean = codeScanner.supported()

    fun link() {
        codeScanner.scan()
    }

    fun stop() {
        codeScanner.cancel()
    }

    @OnEvent
    fun hearScanned(scanned: CodeScanned) {
        linked = scanned.text
    }

    @OnEvent
    fun hearFailed(failed: ScanFailed) {
        failure = failed.reason
    }
}
"#;

    const JAVA: &str = r#"@Service
class StampService {
    private final CodeScanner codeScanner = new CodeScanner();
    String linked = "";
    String failure = "";

    boolean available() {
        return codeScanner.supported();
    }

    void link() {
        codeScanner.scan();
    }

    void stop() {
        codeScanner.cancel();
    }

    @OnEvent
    void hearScanned(CodeScanned scanned) {
        linked = scanned.text();
    }

    @OnEvent
    void hearFailed(ScanFailed failed) {
        failure = failed.reason();
    }
}
"#;

    const DART: &str = r"@Service()
class StampService {
  @Needs()
  final CodeScanner codeScanner;
  String linked = '';
  String failure = '';

  bool available() => codeScanner.supported();

  void link() => codeScanner.scan();

  void stop() => codeScanner.cancel();

  @OnEvent
  void hearScanned(CodeScanned scanned) {
    linked = scanned.text;
  }

  @OnEvent
  void hearFailed(ScanFailed failed) {
    failure = failed.reason;
  }
}
";

    const RUBY: &str = r#"#[Service]
class StampService
  attr_accessor :linked, :failure

  def initialize
    @linked = ""
    @failure = ""
  end

  def available
    CodeScanner.new.supported
  end

  def link
    CodeScanner.new.scan
  end

  def stop
    CodeScanner.new.cancel
  end

  # @param scanned [CodeScanned]
  #[OnEvent]
  def hear_scanned(scanned)
    @linked = scanned.text
  end

  # @param failed [ScanFailed]
  #[OnEvent]
  def hear_failed(failed)
    @failure = failed.reason
  end
end
"#;

    const LUA: &str = r#"-- @Service
---@class StampService
StampService = {
  ---@type string
  linked = "",
  ---@type string
  failure = "",
}

---@return boolean
function StampService:available()
  return CodeScanner:supported()
end

---@return nil
function StampService:link()
  CodeScanner:scan()
end

---@return nil
function StampService:stop()
  CodeScanner:cancel()
end

---@param scanned CodeScanned
---@return nil
-- @OnEvent
function StampService:hear_scanned(scanned)
  self.linked = scanned.text
end

---@param failed ScanFailed
---@return nil
-- @OnEvent
function StampService:hear_failed(failed)
  self.failure = failed.reason
end
"#;

    const CPP: &str = r"#pragma once
#include <string>

class [[Service]] StampService {
public:
    [[Needs]] CodeScanner& codeScanner;
    std::string linked;
    std::string failure;

    bool available() { return codeScanner.supported(); }

    void link() { codeScanner.scan(); }

    void stop() { codeScanner.cancel(); }

    [[OnEvent]] void hearScanned(const CodeScanned& scanned) { linked = scanned.text; }

    [[OnEvent]] void hearFailed(const ScanFailed& failed) { failure = failed.reason; }
};
";

    fn assert_scanned(root: &Path, fetches_files: bool) {
        WebApis::write(
            root,
            "application-events.mjs",
            WebApis::EVENTS_RUNTIME.as_bytes(),
        );
        WebApis::write(root, "tac-code-scanner.mjs", Self::MODULE.as_bytes());
        let scanned = Self::SERVICE_DRIVER.replace("__GRANT_STUBS__", WebApis::GRANT_STUBS);
        let driver = if fetches_files {
            format!("{}{scanned}", WebApis::FILE_FETCH)
        } else {
            scanned
        };
        let report = WebApis::run(root, &driver);
        assert_eq!(
            report,
            serde_json::json!({
                "ready": true,
                "linked": "tachyon-link:demo",
                "denied": "denied",
                "failure": "cancelled",
                "stopped": 2
            })
        );
    }
}

#[tokio::test]
async fn a_rust_web_service_scans_a_code() {
    if !WebApis::node_available() || !WebApis::rust_web_target() {
        eprintln!("skipping: node or the wasm32 Rust target is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Rust, "rs", CodeScannerTests::RUST).await;
    CodeScannerTests::assert_scanned(root.path(), false);
}

#[tokio::test]
async fn a_go_web_service_scans_a_code() {
    let go = Command::new("go")
        .arg("version")
        .output()
        .is_ok_and(|output| output.status.success());
    if !WebApis::node_available() || !go {
        eprintln!("skipping: node or go is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Go, "go", CodeScannerTests::GO).await;
    CodeScannerTests::assert_scanned(root.path(), true);
}

#[tokio::test]
async fn a_csharp_web_service_scans_a_code() {
    if !dotnet_wasm_ready() {
        eprintln!("skipping: node or the .NET wasm-tools workload is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = ("cs", CodeScannerTests::CSHARP);
    let profile = crate::BuildProfile::Development;
    WebApis::publish_with(root.path(), WebLanguage::CSharp, source, profile).await;
    CodeScannerTests::assert_scanned(root.path(), false);
}

#[tokio::test]
async fn an_fsharp_web_service_scans_a_code() {
    if !dotnet_wasm_ready() {
        eprintln!("skipping: node or the .NET wasm-tools workload is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = ("fs", CodeScannerTests::FSHARP);
    let profile = crate::BuildProfile::Development;
    WebApis::publish_with(root.path(), WebLanguage::CSharp, source, profile).await;
    CodeScannerTests::assert_scanned(root.path(), false);
}

#[tokio::test]
async fn a_dart_service_scans_a_code() {
    if !WebApis::node_available() || !WebApis::dart_available() {
        eprintln!("skipping: node or dart is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish_dart(root.path(), CodeScannerTests::DART).await;
    CodeScannerTests::assert_scanned(root.path(), true);
}

#[tokio::test]
async fn a_ruby_web_service_scans_a_code() {
    let unavailable = crate::tac_ruby::RubyRuntimeBuild::unavailable_reason();
    if !WebApis::node_available() || unavailable.is_some() {
        eprintln!("skipping: node or the Ruby runtime is unavailable: {unavailable:?}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Ruby, "rb", CodeScannerTests::RUBY).await;
    CodeScannerTests::assert_scanned(root.path(), true);
}

#[tokio::test]
async fn a_lua_web_service_scans_a_code() {
    if lua_unavailable() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Lua, "lua", CodeScannerTests::LUA).await;
    CodeScannerTests::assert_scanned(root.path(), true);
}

#[tokio::test]
async fn a_swift_web_service_scans_a_code() {
    let root = tempfile::tempdir().expect("project");
    if !publish_swift(root.path(), CodeScannerTests::SWIFT).await {
        return;
    }
    CodeScannerTests::assert_scanned(root.path(), true);
}

#[tokio::test]
async fn a_kotlin_web_service_scans_a_code() {
    if !super::super::kotlin_tests::kotlin_web_ready() {
        eprintln!("skipping: kotlinc-js, the Kotlin wasm stdlib or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(
        root.path(),
        WebLanguage::Kotlin,
        "kt",
        CodeScannerTests::KOTLIN,
    )
    .await;
    CodeScannerTests::assert_scanned(root.path(), true);
}

#[tokio::test]
async fn a_java_web_service_scans_a_code() {
    let root = tempfile::tempdir().expect("project");
    if !publish_java(root.path(), CodeScannerTests::JAVA).await {
        return;
    }
    CodeScannerTests::assert_scanned(root.path(), true);
}

#[tokio::test]
async fn a_cpp_web_service_scans_a_code() {
    if !WebApis::node_available() || !WebApis::cpp_available() {
        eprintln!("skipping: node or em++ is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Cpp, "hpp", CodeScannerTests::CPP).await;
    CodeScannerTests::assert_scanned(root.path(), true);
}
