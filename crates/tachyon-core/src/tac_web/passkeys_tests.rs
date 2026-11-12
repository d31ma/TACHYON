use super::{
    WebApis, WebLanguage, dotnet_wasm_ready, lua_unavailable, publish_java, publish_swift,
};
use std::path::Path;
use std::process::Command;

struct PasskeysTests;

impl PasskeysTests {
    const MODULE: &str = include_str!("../template/tac-passkeys.js");

    const SERVICE_DRIVER: &str = r";(0, eval)(`${await readFile('./tac-web.js', 'utf8')}\nglobalThis.TacWebApis = TacWebApis`)
;(0, eval)(await readFile('./tac-passkeys.js', 'utf8'))
const { default: TacApplicationEvents } = await import('./application-events.mjs')
const { default: StampService, __tachyon_event_plan__: plan } = await import('./StampService.js')
__GRANT_STUBS__
grants.content = 'passkeys'
globalThis.isSecureContext = true
const asked = []
globalThis.PublicKeyCredential = class {
  static parseCreationOptionsFromJSON = (json) => ({ ...json, challenge: new Uint8Array([1]) })
  static parseRequestOptionsFromJSON = (json) => ({ ...json, challenge: new Uint8Array([2]) })
}
navigator.credentials = {
  create: async (options) => { asked.push(['create', [...options.publicKey.challenge]]); return { toJSON: () => ({ id: 'AQ', type: 'public-key' }) } },
  get: async (options) => { asked.push(['get', [...options.publicKey.challenge]]); throw new DOMException('closed by the person', 'NotAllowedError') },
}
const settle = async () => { for (let round = 0; round < 10; round += 1) await new Promise((resolve) => setTimeout(resolve, 0)) }
const bus = new TacApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })
TacWebApis.events = { publish: (name, value) => bus.publish(name, value), send: (owner, name, value) => bus.send(owner, name, value) }
const stamp = new StampService({})
bus.attach(stamp, plan.types[0], { applicationEventPublisher: null })
await bus.initialize(stamp)
const ready = stamp.available()
stamp.enroll()
await settle()
stamp.sign()
await settle()
console.log(JSON.stringify({ ready, created: stamp.created, failure: stamp.failure, asked }))
process.exit(0)
";

    const RUST: &str = r#"#[Service]
#[derive(Default)]
struct StampService {
    passkeys: Passkeys,
    created: String,
    failure: String,
}

impl StampService {
    pub fn available(&self) -> bool {
        self.passkeys.supported().unwrap_or(false)
    }

    pub fn enroll(&self) {
        let _ = self.passkeys.create("{}");
    }

    pub fn sign(&self) {
        let _ = self.passkeys.get("{}");
    }

    #[OnEvent]
    pub fn hear_created(&mut self, created: PasskeyCreated) {
        self.created = created.credentialJson;
    }

    #[OnEvent]
    pub fn hear_failed(&mut self, failed: PasskeyFailed) {
        self.failure = format!("{} {}", failed.call, failed.reason);
    }
}
"#;

    const GO: &str = r#"// @Service
type StampService struct {
	// @Needs
	passkeys Passkeys
	Created  string
	Failure  string
}

func (stamp *StampService) Available() bool {
	supported, _ := stamp.passkeys.Supported()
	return supported
}

func (stamp *StampService) Enroll() {
	_ = stamp.passkeys.Create("{}")
}

func (stamp *StampService) Sign() {
	_ = stamp.passkeys.Get("{}")
}

// @OnEvent
func (stamp *StampService) HearCreated(created PasskeyCreated) {
	stamp.Created = created.CredentialJson
}

// @OnEvent
func (stamp *StampService) HearFailed(failed PasskeyFailed) {
	stamp.Failure = failed.Call + " " + failed.Reason
}
"#;

    const CSHARP: &str = r#"[Service]
public class StampService
{
    private readonly Passkeys passkeys = new();
    public string Created = "";
    public string Failure = "";

    public bool Available() => passkeys.Supported();

    public void Enroll() => passkeys.Create("{}");

    public void Sign() => passkeys.Get("{}");

    [OnEvent]
    public void HearCreated(PasskeyCreated created)
    {
        Created = created.CredentialJson;
    }

    [OnEvent]
    public void HearFailed(PasskeyFailed failed)
    {
        Failure = failed.Call + " " + failed.Reason;
    }
}
"#;

    const FSHARP: &str = r#"[<Service>]
type StampService() =
    let passkeys = Passkeys()
    member val Created: string = "" with get, set
    member val Failure: string = "" with get, set
    member this.Available() : bool = passkeys.Supported()
    member this.Enroll() : unit = passkeys.Create("{}")
    member this.Sign() : unit = passkeys.Get("{}")
    [<OnEvent>]
    member this.HearCreated(created: PasskeyCreated) : unit = this.Created <- created.CredentialJson
    [<OnEvent>]
    member this.HearFailed(failed: PasskeyFailed) : unit = this.Failure <- failed.Call + " " + failed.Reason
"#;

    const SWIFT: &str = r#"@Service
final class StampService {
    private let passkeys = Passkeys()
    var created: String = ""
    var failure: String = ""

    func available() -> Bool {
        (try? passkeys.supported()) ?? false
    }

    func enroll() throws {
        try passkeys.create(optionsJson: "{}")
    }

    func sign() throws {
        try passkeys.get(optionsJson: "{}")
    }

    @OnEvent
    func hearCreated(_ passkey: PasskeyCreated) {
        created = passkey.credentialJson
    }

    @OnEvent
    func hearFailed(_ failed: PasskeyFailed) {
        failure = "\(failed.call) \(failed.reason)"
    }
}
"#;

    const KOTLIN: &str = r#"@Service
class StampService {
    private val passkeys = Passkeys()
    var created: String = ""
    var failure: String = ""

    fun available(): Boolean = passkeys.supported()

    fun enroll() {
        passkeys.create("{}")
    }

    fun sign() {
        passkeys.get("{}")
    }

    @OnEvent
    fun hearCreated(passkey: PasskeyCreated) {
        created = passkey.credentialJson
    }

    @OnEvent
    fun hearFailed(failed: PasskeyFailed) {
        failure = failed.call + " " + failed.reason
    }
}
"#;

    const JAVA: &str = r#"@Service
class StampService {
    private final Passkeys passkeys = new Passkeys();
    String created = "";
    String failure = "";

    boolean available() {
        return passkeys.supported();
    }

    void enroll() {
        passkeys.create("{}");
    }

    void sign() {
        passkeys.get("{}");
    }

    @OnEvent
    void hearCreated(PasskeyCreated passkey) {
        created = passkey.credentialJson();
    }

    @OnEvent
    void hearFailed(PasskeyFailed failed) {
        failure = failed.call() + " " + failed.reason();
    }
}
"#;

    const DART: &str = r"@Service()
class StampService {
  @Needs()
  final Passkeys passkeys;
  String created = '';
  String failure = '';

  bool available() => passkeys.supported();

  void enroll() => passkeys.create('{}');

  void sign() => passkeys.get('{}');

  @OnEvent
  void hearCreated(PasskeyCreated passkey) {
    created = passkey.credentialJson;
  }

  @OnEvent
  void hearFailed(PasskeyFailed failed) {
    failure = '${failed.call} ${failed.reason}';
  }
}
";

    const RUBY: &str = r#"#[Service]
class StampService
  attr_accessor :created, :failure

  def initialize
    @created = ""
    @failure = ""
  end

  def available
    Passkeys.new.supported
  end

  def enroll
    Passkeys.new.create("{}")
  end

  def sign
    Passkeys.new.get("{}")
  end

  # @param passkey [PasskeyCreated]
  #[OnEvent]
  def hear_created(passkey)
    @created = passkey.credentialJson
  end

  # @param failed [PasskeyFailed]
  #[OnEvent]
  def hear_failed(failed)
    @failure = [failed.call, failed.reason].join(" ")
  end
end
"#;

    const LUA: &str = r#"-- @Service
---@class StampService
StampService = {
  ---@type string
  created = "",
  ---@type string
  failure = "",
}

---@return boolean
function StampService:available()
  return Passkeys:supported()
end

---@return nil
function StampService:enroll()
  Passkeys:create("{}")
end

---@return nil
function StampService:sign()
  Passkeys:get("{}")
end

---@param passkey PasskeyCreated
---@return nil
-- @OnEvent
function StampService:hear_created(passkey)
  self.created = passkey.credentialJson
end

---@param failed PasskeyFailed
---@return nil
-- @OnEvent
function StampService:hear_failed(failed)
  self.failure = failed.call .. " " .. failed.reason
end
"#;

    const CPP: &str = r#"#pragma once
#include <string>

class [[Service]] StampService {
public:
    [[Needs]] Passkeys& passkeys;
    std::string created;
    std::string failure;

    bool available() { return passkeys.supported(); }

    void enroll() { passkeys.create("{}"); }

    void sign() { passkeys.get("{}"); }

    [[OnEvent]] void hearCreated(const PasskeyCreated& passkey) { created = passkey.credentialJson; }

    [[OnEvent]] void hearFailed(const PasskeyFailed& failed) { failure = failed.call + " " + failed.reason; }
};
"#;

    fn assert_signed(root: &Path, fetches_files: bool) {
        WebApis::write(
            root,
            "application-events.mjs",
            WebApis::EVENTS_RUNTIME.as_bytes(),
        );
        WebApis::write(root, "tac-passkeys.js", Self::MODULE.as_bytes());
        let signed = Self::SERVICE_DRIVER.replace("__GRANT_STUBS__", WebApis::GRANT_STUBS);
        let driver = if fetches_files {
            format!("{}{signed}", WebApis::FILE_FETCH)
        } else {
            signed
        };
        let report = WebApis::run(root, &driver);
        assert_eq!(
            report,
            serde_json::json!({
                "ready": true,
                "created": "{\"id\":\"AQ\",\"type\":\"public-key\"}",
                "failure": "get cancelled",
                "asked": [["create", [1]], ["get", [2]]]
            })
        );
    }
}

#[tokio::test]
async fn a_rust_web_service_creates_and_uses_passkeys() {
    if !WebApis::node_available() || !WebApis::rust_web_target() {
        eprintln!("skipping: node or the wasm32 Rust target is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Rust, "rs", PasskeysTests::RUST).await;
    PasskeysTests::assert_signed(root.path(), false);
}

#[tokio::test]
async fn a_go_web_service_creates_and_uses_passkeys() {
    let go = Command::new("go")
        .arg("version")
        .output()
        .is_ok_and(|output| output.status.success());
    if !WebApis::node_available() || !go {
        eprintln!("skipping: node or go is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Go, "go", PasskeysTests::GO).await;
    PasskeysTests::assert_signed(root.path(), true);
}

#[tokio::test]
async fn a_csharp_web_service_creates_and_uses_passkeys() {
    if !dotnet_wasm_ready() {
        eprintln!("skipping: node or the .NET wasm-tools workload is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = ("cs", PasskeysTests::CSHARP);
    let profile = crate::BuildProfile::Development;
    WebApis::publish_with(root.path(), WebLanguage::CSharp, source, profile).await;
    PasskeysTests::assert_signed(root.path(), false);
}

#[tokio::test]
async fn an_fsharp_web_service_creates_and_uses_passkeys() {
    if !dotnet_wasm_ready() {
        eprintln!("skipping: node or the .NET wasm-tools workload is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = ("fs", PasskeysTests::FSHARP);
    let profile = crate::BuildProfile::Development;
    WebApis::publish_with(root.path(), WebLanguage::CSharp, source, profile).await;
    PasskeysTests::assert_signed(root.path(), false);
}

#[tokio::test]
async fn a_dart_service_creates_and_uses_passkeys() {
    if !WebApis::node_available() || !WebApis::dart_available() {
        eprintln!("skipping: node or dart is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish_dart(root.path(), PasskeysTests::DART).await;
    PasskeysTests::assert_signed(root.path(), true);
}

#[tokio::test]
async fn a_ruby_web_service_creates_and_uses_passkeys() {
    let unavailable = crate::tac_ruby::RubyRuntimeBuild::unavailable_reason();
    if !WebApis::node_available() || unavailable.is_some() {
        eprintln!("skipping: node or the Ruby runtime is unavailable: {unavailable:?}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Ruby, "rb", PasskeysTests::RUBY).await;
    PasskeysTests::assert_signed(root.path(), true);
}

#[tokio::test]
async fn a_lua_web_service_creates_and_uses_passkeys() {
    if lua_unavailable() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Lua, "lua", PasskeysTests::LUA).await;
    PasskeysTests::assert_signed(root.path(), true);
}

#[tokio::test]
async fn a_swift_web_service_creates_and_uses_passkeys() {
    let root = tempfile::tempdir().expect("project");
    if !publish_swift(root.path(), PasskeysTests::SWIFT).await {
        return;
    }
    PasskeysTests::assert_signed(root.path(), true);
}

#[tokio::test]
async fn a_kotlin_web_service_creates_and_uses_passkeys() {
    if !super::super::kotlin_tests::kotlin_web_ready() {
        eprintln!("skipping: kotlinc-js, the Kotlin wasm stdlib or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(
        root.path(),
        WebLanguage::Kotlin,
        "kt",
        PasskeysTests::KOTLIN,
    )
    .await;
    PasskeysTests::assert_signed(root.path(), true);
}

#[tokio::test]
async fn a_java_web_service_creates_and_uses_passkeys() {
    let root = tempfile::tempdir().expect("project");
    if !publish_java(root.path(), PasskeysTests::JAVA).await {
        return;
    }
    PasskeysTests::assert_signed(root.path(), true);
}

#[tokio::test]
async fn a_cpp_web_service_creates_and_uses_passkeys() {
    if !WebApis::node_available() || !WebApis::cpp_available() {
        eprintln!("skipping: node or em++ is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Cpp, "hpp", PasskeysTests::CPP).await;
    PasskeysTests::assert_signed(root.path(), true);
}
