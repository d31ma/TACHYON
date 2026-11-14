use super::{
    WebApis, WebLanguage, dotnet_wasm_ready, lua_unavailable, publish_java, publish_swift,
};
use std::path::Path;
use std::process::Command;

struct CryptoTests;

impl CryptoTests {
    const MODULE: &str = include_str!("../template/tac-crypto.js");

    const SERVICE_DRIVER: &str = r";(0, eval)(`${await readFile('./tac-web.js', 'utf8')}\nglobalThis.TacWebApis = TacWebApis`)
;(0, eval)(await readFile('./tac-crypto.js', 'utf8'))
const { default: TacApplicationEvents } = await import('./application-events.mjs')
const { default: StampService, __tachyon_event_plan__: plan } = await import('./StampService.js')
__GRANT_STUBS__
grants.content = 'crypto'
globalThis.isSecureContext = true
const settle = async () => { for (let round = 0; round < 50; round += 1) await new Promise((resolve) => setTimeout(resolve, 2)) }
const bus = new TacApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })
TacWebApis.events = { publish: (name, value) => bus.publish(name, value), send: (owner, name, value) => bus.send(owner, name, value) }
const stamp = new StampService({})
bus.attach(stamp, plan.types[0], { applicationEventPublisher: null })
await bus.initialize(stamp)
const ready = stamp.available()
stamp.start()
await settle()
console.log(JSON.stringify({ ready, request: stamp.request, made: stamp.made, derived: stamp.derived, failure: stamp.failure }))
process.exit(0)
";

    const RUST: &str = r#"#[Service]
#[derive(Default)]
struct StampService {
    crypto: Crypto,
    request: String,
    made: String,
    derived: String,
    failure: String,
}

impl StampService {
    const PEER: &'static str = "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE";

    pub fn available(&self) -> bool {
        self.crypto.supported().unwrap_or(false)
    }

    pub fn start(&mut self) {
        self.request = self.crypto.make_key_pair("device", "ed25519", false).unwrap_or_default();
        let _ = self.crypto.hkdf("AQI", "", "", 4);
        let _ = self.crypto.agree("device", Self::PEER, "", "", "shared", "encrypt", false);
    }

    #[OnEvent]
    pub fn hear_made(&mut self, made: CryptoKeyMade) {
        self.made = made.key;
    }

    #[OnEvent]
    pub fn hear_derived(&mut self, derived: CryptoDerived) {
        self.derived = derived.bytes;
    }

    #[OnEvent]
    pub fn hear_failed(&mut self, failed: CryptoFailed) {
        self.failure = format!("{} {}", failed.call, failed.reason);
    }
}
"#;

    const GO: &str = r#"// @Service
type StampService struct {
	// @Needs
	crypto  Crypto
	Request string
	Made    string
	Derived string
	Failure string
}

func (stamp *StampService) Available() bool {
	supported, _ := stamp.crypto.Supported()
	return supported
}

func (stamp *StampService) Start() {
	stamp.Request, _ = stamp.crypto.MakeKeyPair("device", "ed25519", false)
	_, _ = stamp.crypto.Hkdf("AQI", "", "", 4)
	_, _ = stamp.crypto.Agree("device", "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE", "", "", "shared", "encrypt", false)
}

// @OnEvent
func (stamp *StampService) HearMade(made CryptoKeyMade) {
	stamp.Made = made.Key
}

// @OnEvent
func (stamp *StampService) HearDerived(derived CryptoDerived) {
	stamp.Derived = derived.Bytes
}

// @OnEvent
func (stamp *StampService) HearFailed(failed CryptoFailed) {
	stamp.Failure = failed.Call + " " + failed.Reason
}
"#;

    const CSHARP: &str = r#"[Service]
public class StampService
{
    private const string Peer = "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE";
    private readonly Crypto crypto = new();
    public string Request = "";
    public string Made = "";
    public string Derived = "";
    public string Failure = "";

    public bool Available() => crypto.Supported();

    public void Start()
    {
        Request = crypto.MakeKeyPair("device", "ed25519", false);
        crypto.Hkdf("AQI", "", "", 4);
        crypto.Agree("device", Peer, "", "", "shared", "encrypt", false);
    }

    [OnEvent]
    public void HearMade(CryptoKeyMade made)
    {
        Made = made.Key;
    }

    [OnEvent]
    public void HearDerived(CryptoDerived derived)
    {
        Derived = derived.Bytes;
    }

    [OnEvent]
    public void HearFailed(CryptoFailed failed)
    {
        Failure = failed.Call + " " + failed.Reason;
    }
}
"#;

    const FSHARP: &str = r#"[<Service>]
type StampService() =
    let crypto = Crypto()
    member val Request: string = "" with get, set
    member val Made: string = "" with get, set
    member val Derived: string = "" with get, set
    member val Failure: string = "" with get, set
    member this.Available() : bool = crypto.Supported()
    member this.Start() : unit =
        this.Request <- crypto.MakeKeyPair("device", "ed25519", false)
        crypto.Hkdf("AQI", "", "", 4) |> ignore
        crypto.Agree("device", "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE", "", "", "shared", "encrypt", false) |> ignore
    [<OnEvent>]
    member this.HearMade(made: CryptoKeyMade) : unit = this.Made <- made.Key
    [<OnEvent>]
    member this.HearDerived(derived: CryptoDerived) : unit = this.Derived <- derived.Bytes
    [<OnEvent>]
    member this.HearFailed(failed: CryptoFailed) : unit = this.Failure <- failed.Call + " " + failed.Reason
"#;

    const SWIFT: &str = r#"@Service
final class StampService {
    private static let peer = "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE"
    private let crypto = Crypto()
    var request: String = ""
    var made: String = ""
    var derived: String = ""
    var failure: String = ""

    func available() -> Bool {
        (try? crypto.supported()) ?? false
    }

    func start() throws {
        request = try crypto.makeKeyPair(key: "device", algorithm: "ed25519", keep: false)
        _ = try crypto.hkdf(input: "AQI", salt: "", info: "", length: 4)
        _ = try crypto.agree(key: "device", peerPublicKey: Self.peer, salt: "", info: "", newKey: "shared", use: "encrypt", keep: false)
    }

    @OnEvent
    func hearMade(_ made: CryptoKeyMade) {
        self.made = made.key
    }

    @OnEvent
    func hearDerived(_ derived: CryptoDerived) {
        self.derived = derived.bytes
    }

    @OnEvent
    func hearFailed(_ failed: CryptoFailed) {
        failure = "\(failed.call) \(failed.reason)"
    }
}
"#;

    const KOTLIN: &str = r#"@Service
class StampService {
    private val crypto = Crypto()
    private val peer = "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE"
    var request: String = ""
    var made: String = ""
    var derived: String = ""
    var failure: String = ""

    fun available(): Boolean = crypto.supported()

    fun start() {
        request = crypto.makeKeyPair("device", "ed25519", false)
        crypto.hkdf("AQI", "", "", 4)
        crypto.agree("device", peer, "", "", "shared", "encrypt", false)
    }

    @OnEvent
    fun hearMade(made: CryptoKeyMade) {
        this.made = made.key
    }

    @OnEvent
    fun hearDerived(derived: CryptoDerived) {
        this.derived = derived.bytes
    }

    @OnEvent
    fun hearFailed(failed: CryptoFailed) {
        failure = failed.call + " " + failed.reason
    }
}
"#;

    const JAVA: &str = r#"@Service
class StampService {
    private static final String PEER = "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE";
    private final Crypto crypto = new Crypto();
    String request = "";
    String made = "";
    String derived = "";
    String failure = "";

    boolean available() {
        return crypto.supported();
    }

    void start() {
        request = crypto.makeKeyPair("device", "ed25519", false);
        crypto.hkdf("AQI", "", "", 4);
        crypto.agree("device", PEER, "", "", "shared", "encrypt", false);
    }

    @OnEvent
    void hearMade(CryptoKeyMade made) {
        this.made = made.key();
    }

    @OnEvent
    void hearDerived(CryptoDerived derived) {
        this.derived = derived.bytes();
    }

    @OnEvent
    void hearFailed(CryptoFailed failed) {
        failure = failed.call() + " " + failed.reason();
    }
}
"#;

    const DART: &str = r"@Service()
class StampService {
  static const _peer = 'AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE';
  @Needs()
  final Crypto crypto;
  String request = '';
  String made = '';
  String derived = '';
  String failure = '';

  bool available() => crypto.supported();

  void start() {
    request = crypto.makeKeyPair('device', 'ed25519', false);
    crypto.hkdf('AQI', '', '', 4);
    crypto.agree('device', _peer, '', '', 'shared', 'encrypt', false);
  }

  @OnEvent
  void hearMade(CryptoKeyMade made) {
    this.made = made.key;
  }

  @OnEvent
  void hearDerived(CryptoDerived derived) {
    this.derived = derived.bytes;
  }

  @OnEvent
  void hearFailed(CryptoFailed failed) {
    failure = '${failed.call} ${failed.reason}';
  }
}
";

    const RUBY: &str = r#"#[Service]
class StampService
  PEER = "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE"

  attr_accessor :request, :made, :derived, :failure

  def initialize
    @request = ""
    @made = ""
    @derived = ""
    @failure = ""
  end

  def available
    Crypto.new.supported
  end

  def start
    @request = Crypto.new.make_key_pair("device", "ed25519", false)
    Crypto.new.hkdf("AQI", "", "", 4)
    Crypto.new.agree("device", PEER, "", "", "shared", "encrypt", false)
  end

  # @param made [CryptoKeyMade]
  #[OnEvent]
  def hear_made(made)
    @made = made.key
  end

  # @param derived [CryptoDerived]
  #[OnEvent]
  def hear_derived(derived)
    @derived = derived.bytes
  end

  # @param failed [CryptoFailed]
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
  request = "",
  ---@type string
  made = "",
  ---@type string
  derived = "",
  ---@type string
  failure = "",
}

---@return boolean
function StampService:available()
  return Crypto:supported()
end

---@return nil
function StampService:start()
  self.request = Crypto:make_key_pair("device", "ed25519", false)
  Crypto:hkdf("AQI", "", "", 4)
  Crypto:agree("device", "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE", "", "", "shared", "encrypt", false)
end

---@param made CryptoKeyMade
---@return nil
-- @OnEvent
function StampService:hear_made(made)
  self.made = made.key
end

---@param derived CryptoDerived
---@return nil
-- @OnEvent
function StampService:hear_derived(derived)
  self.derived = derived.bytes
end

---@param failed CryptoFailed
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
    [[Needs]] Crypto& crypto;
    std::string request;
    std::string made;
    std::string derived;
    std::string failure;

    bool available() { return crypto.supported(); }

    void start() {
        request = crypto.makeKeyPair("device", "ed25519", false);
        crypto.hkdf("AQI", "", "", 4);
        crypto.agree("device", "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE", "", "", "shared", "encrypt", false);
    }

    [[OnEvent]] void hearMade(const CryptoKeyMade& event) { made = event.key; }

    [[OnEvent]] void hearDerived(const CryptoDerived& event) { derived = event.bytes; }

    [[OnEvent]] void hearFailed(const CryptoFailed& failed) { failure = failed.call + " " + failed.reason; }
};
"#;

    fn assert_linked(root: &Path, fetches_files: bool) {
        WebApis::write(
            root,
            "application-events.mjs",
            WebApis::EVENTS_RUNTIME.as_bytes(),
        );
        WebApis::write(root, "tac-crypto.js", Self::MODULE.as_bytes());
        let linked = Self::SERVICE_DRIVER.replace("__GRANT_STUBS__", WebApis::GRANT_STUBS);
        let driver = if fetches_files {
            format!("{}{linked}", WebApis::FILE_FETCH)
        } else {
            linked
        };
        let report = WebApis::run(root, &driver);
        assert_eq!(
            report,
            serde_json::json!({
                "ready": true,
                "request": "crypto-1",
                "made": "device",
                "derived": "ILbgBA",
                "failure": "agree invalid"
            })
        );
    }
}

#[tokio::test]
async fn a_rust_web_service_signs_and_derives_with_crypto() {
    if !WebApis::node_available() || !WebApis::rust_web_target() {
        eprintln!("skipping: node or the wasm32 Rust target is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Rust, "rs", CryptoTests::RUST).await;
    CryptoTests::assert_linked(root.path(), false);
}

#[tokio::test]
async fn a_go_web_service_signs_and_derives_with_crypto() {
    let go = Command::new("go")
        .arg("version")
        .output()
        .is_ok_and(|output| output.status.success());
    if !WebApis::node_available() || !go {
        eprintln!("skipping: node or go is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Go, "go", CryptoTests::GO).await;
    CryptoTests::assert_linked(root.path(), true);
}

#[tokio::test]
async fn a_csharp_web_service_signs_and_derives_with_crypto() {
    if !dotnet_wasm_ready() {
        eprintln!("skipping: node or the .NET wasm-tools workload is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = ("cs", CryptoTests::CSHARP);
    let profile = crate::BuildProfile::Development;
    WebApis::publish_with(root.path(), WebLanguage::CSharp, source, profile).await;
    CryptoTests::assert_linked(root.path(), false);
}

#[tokio::test]
async fn an_fsharp_web_service_signs_and_derives_with_crypto() {
    if !dotnet_wasm_ready() {
        eprintln!("skipping: node or the .NET wasm-tools workload is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = ("fs", CryptoTests::FSHARP);
    let profile = crate::BuildProfile::Development;
    WebApis::publish_with(root.path(), WebLanguage::CSharp, source, profile).await;
    CryptoTests::assert_linked(root.path(), false);
}

#[tokio::test]
async fn a_dart_service_signs_and_derives_with_crypto() {
    if !WebApis::node_available() || !WebApis::dart_available() {
        eprintln!("skipping: node or dart is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish_dart(root.path(), CryptoTests::DART).await;
    CryptoTests::assert_linked(root.path(), true);
}

#[tokio::test]
async fn a_ruby_web_service_signs_and_derives_with_crypto() {
    let unavailable = crate::tac_ruby::RubyRuntimeBuild::unavailable_reason();
    if !WebApis::node_available() || unavailable.is_some() {
        eprintln!("skipping: node or the Ruby runtime is unavailable: {unavailable:?}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Ruby, "rb", CryptoTests::RUBY).await;
    CryptoTests::assert_linked(root.path(), true);
}

#[tokio::test]
async fn a_lua_web_service_signs_and_derives_with_crypto() {
    if lua_unavailable() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Lua, "lua", CryptoTests::LUA).await;
    CryptoTests::assert_linked(root.path(), true);
}

#[tokio::test]
async fn a_swift_web_service_signs_and_derives_with_crypto() {
    let root = tempfile::tempdir().expect("project");
    if !publish_swift(root.path(), CryptoTests::SWIFT).await {
        return;
    }
    CryptoTests::assert_linked(root.path(), true);
}

#[tokio::test]
async fn a_kotlin_web_service_signs_and_derives_with_crypto() {
    if !super::super::kotlin_tests::kotlin_web_ready() {
        eprintln!("skipping: kotlinc-js, the Kotlin wasm stdlib or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Kotlin, "kt", CryptoTests::KOTLIN).await;
    CryptoTests::assert_linked(root.path(), true);
}

#[tokio::test]
async fn a_java_web_service_signs_and_derives_with_crypto() {
    let root = tempfile::tempdir().expect("project");
    if !publish_java(root.path(), CryptoTests::JAVA).await {
        return;
    }
    CryptoTests::assert_linked(root.path(), true);
}

#[tokio::test]
async fn a_cpp_web_service_signs_and_derives_with_crypto() {
    if !WebApis::node_available() || !WebApis::cpp_available() {
        eprintln!("skipping: node or em++ is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Cpp, "hpp", CryptoTests::CPP).await;
    CryptoTests::assert_linked(root.path(), true);
}
