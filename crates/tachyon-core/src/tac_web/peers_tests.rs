use super::{
    WebApis, WebLanguage, dotnet_wasm_ready, lua_unavailable, publish_java, publish_swift,
};
use std::path::Path;
use std::process::Command;

struct PeersTests;

impl PeersTests {
    const MODULE: &str = include_str!("../template/tac-peers.js");

    const SERVICE_DRIVER: &str = r";(0, eval)(`${await readFile('./tac-web.js', 'utf8')}\nglobalThis.TacWebApis = TacWebApis`)
;(0, eval)(await readFile('./tac-peers.js', 'utf8'))
const { default: TacApplicationEvents } = await import('./application-events.mjs')
const { default: StampService, __tachyon_event_plan__: plan } = await import('./StampService.js')
__GRANT_STUBS__
grants.content = 'peers'
const channels = []
class FakeChannel extends EventTarget {
  constructor(label, options) { super(); this.label = label; this.ordered = options.ordered; this.readyState = 'connecting'; this.bufferedAmount = 0; this.sent = [] }
  send(bytes) { this.sent.push([...bytes]) }
  close() {}
}
globalThis.RTCPeerConnection = class extends EventTarget {
  async createOffer() { return { type: 'offer', sdp: 'v=0 offer' } }
  async setLocalDescription(description) { this.localDescription = description }
  async setRemoteDescription() { throw new DOMException('rp https://secret.example.com', 'OperationError') }
  createDataChannel(label, options) { const channel = new FakeChannel(label, options); channels.push(channel); return channel }
  close() {}
}
const settle = async () => { for (let round = 0; round < 10; round += 1) await new Promise((resolve) => setTimeout(resolve, 0)) }
const bus = new TacApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })
TacWebApis.events = { publish: (name, value) => bus.publish(name, value), send: (owner, name, value) => bus.send(owner, name, value) }
const link = new StampService({})
bus.attach(link, plan.types[0], { applicationEventPublisher: null })
await bus.initialize(link)
const ready = link.available()
link.start()
await settle()
channels[0].readyState = 'open'
channels[0].dispatchEvent(new Event('open'))
await settle()
link.push()
const message = new Event('message')
message.data = new Uint8Array([251, 255]).buffer
channels[0].dispatchEvent(message)
await settle()
link.refuse()
await settle()
console.log(JSON.stringify({ ready, connection: link.connection, channel: link.channel, queued: link.queued, description: link.description, message: link.message, failure: link.failure, sent: channels[0].sent }))
process.exit(0)
";

    const RUST: &str = r#"#[Service]
#[derive(Default)]
struct StampService {
    peers: Peers,
    connection: String,
    channel: String,
    queued: bool,
    description: String,
    message: String,
    failure: String,
}

impl StampService {
    pub fn available(&self) -> bool {
        self.peers.supported().unwrap_or(false)
    }

    pub fn start(&mut self) {
        self.connection = self.peers.open("", "").unwrap_or_default();
        let _ = self.peers.offer(&self.connection);
        self.channel = self.peers.open_channel(&self.connection, "chat", true).unwrap_or_default();
    }

    pub fn push(&mut self) {
        self.queued = self.peers.send(&self.connection, &self.channel, "AQI").unwrap_or(false);
    }

    pub fn refuse(&self) {
        let _ = self.peers.accept(&self.connection, "bad");
    }

    #[OnEvent]
    pub fn hear_description(&mut self, described: PeerDescription) {
        self.description = format!("{} {}", described.kind, described.sdp);
    }

    #[OnEvent]
    pub fn hear_message(&mut self, received: PeerMessage) {
        self.message = received.data;
    }

    #[OnEvent]
    pub fn hear_failed(&mut self, failed: PeerFailed) {
        self.failure = format!("{} {}", failed.call, failed.reason);
    }
}
"#;

    const GO: &str = r#"// @Service
type StampService struct {
	// @Needs
	peers       Peers
	Connection  string
	Channel     string
	Queued      bool
	Description string
	Message     string
	Failure     string
}

func (link *StampService) Available() bool {
	supported, _ := link.peers.Supported()
	return supported
}

func (link *StampService) Start() {
	link.Connection, _ = link.peers.Open("", "")
	_ = link.peers.Offer(link.Connection)
	link.Channel, _ = link.peers.OpenChannel(link.Connection, "chat", true)
}

func (link *StampService) Push() {
	link.Queued, _ = link.peers.Send(link.Connection, link.Channel, "AQI")
}

func (link *StampService) Refuse() {
	_ = link.peers.Accept(link.Connection, "bad")
}

// @OnEvent
func (link *StampService) HearDescription(described PeerDescription) {
	link.Description = described.Kind + " " + described.Sdp
}

// @OnEvent
func (link *StampService) HearMessage(received PeerMessage) {
	link.Message = received.Data
}

// @OnEvent
func (link *StampService) HearFailed(failed PeerFailed) {
	link.Failure = failed.Call + " " + failed.Reason
}
"#;

    const CSHARP: &str = r#"[Service]
public class StampService
{
    private readonly Peers peers = new();
    public string Connection = "";
    public string Channel = "";
    public bool Queued;
    public string Description = "";
    public string Message = "";
    public string Failure = "";

    public bool Available() => peers.Supported();

    public void Start()
    {
        Connection = peers.Open("", "");
        peers.Offer(Connection);
        Channel = peers.OpenChannel(Connection, "chat", true);
    }

    public void Push() => Queued = peers.Send(Connection, Channel, "AQI");

    public void Refuse() => peers.Accept(Connection, "bad");

    [OnEvent]
    public void HearDescription(PeerDescription described)
    {
        Description = described.Kind + " " + described.Sdp;
    }

    [OnEvent]
    public void HearMessage(PeerMessage received)
    {
        Message = received.Data;
    }

    [OnEvent]
    public void HearFailed(PeerFailed failed)
    {
        Failure = failed.Call + " " + failed.Reason;
    }
}
"#;

    const FSHARP: &str = r#"[<Service>]
type StampService() =
    let peers = Peers()
    member val Connection: string = "" with get, set
    member val Channel: string = "" with get, set
    member val Queued: bool = false with get, set
    member val Description: string = "" with get, set
    member val Message: string = "" with get, set
    member val Failure: string = "" with get, set
    member this.Available() : bool = peers.Supported()
    member this.Start() : unit =
        this.Connection <- peers.Open("", "")
        peers.Offer(this.Connection)
        this.Channel <- peers.OpenChannel(this.Connection, "chat", true)
    member this.Push() : unit = this.Queued <- peers.Send(this.Connection, this.Channel, "AQI")
    member this.Refuse() : unit = peers.Accept(this.Connection, "bad")
    [<OnEvent>]
    member this.HearDescription(described: PeerDescription) : unit = this.Description <- described.Kind + " " + described.Sdp
    [<OnEvent>]
    member this.HearMessage(received: PeerMessage) : unit = this.Message <- received.Data
    [<OnEvent>]
    member this.HearFailed(failed: PeerFailed) : unit = this.Failure <- failed.Call + " " + failed.Reason
"#;

    const SWIFT: &str = r#"@Service
final class StampService {
    private let peers = Peers()
    var connection: String = ""
    var channel: String = ""
    var queued: Bool = false
    var description: String = ""
    var message: String = ""
    var failure: String = ""

    func available() -> Bool {
        (try? peers.supported()) ?? false
    }

    func start() throws {
        connection = try peers.open(username: "", credential: "")
        try peers.offer(connection: connection)
        channel = try peers.openChannel(connection: connection, label: "chat", ordered: true)
    }

    func push() throws {
        queued = try peers.send(connection: connection, channel: channel, data: "AQI")
    }

    func refuse() throws {
        try peers.accept(connection: connection, answerSdp: "bad")
    }

    @OnEvent
    func hearDescription(_ described: PeerDescription) {
        description = "\(described.kind) \(described.sdp)"
    }

    @OnEvent
    func hearMessage(_ received: PeerMessage) {
        message = received.data
    }

    @OnEvent
    func hearFailed(_ failed: PeerFailed) {
        failure = "\(failed.call) \(failed.reason)"
    }
}
"#;

    const KOTLIN: &str = r#"@Service
class StampService {
    private val peers = Peers()
    var connection: String = ""
    var channel: String = ""
    var queued: Boolean = false
    var description: String = ""
    var message: String = ""
    var failure: String = ""

    fun available(): Boolean = peers.supported()

    fun start() {
        connection = peers.open("", "")
        peers.offer(connection)
        channel = peers.openChannel(connection, "chat", true)
    }

    fun push() {
        queued = peers.send(connection, channel, "AQI")
    }

    fun refuse() {
        peers.accept(connection, "bad")
    }

    @OnEvent
    fun hearDescription(described: PeerDescription) {
        description = described.kind + " " + described.sdp
    }

    @OnEvent
    fun hearMessage(received: PeerMessage) {
        message = received.data
    }

    @OnEvent
    fun hearFailed(failed: PeerFailed) {
        failure = failed.call + " " + failed.reason
    }
}
"#;

    const JAVA: &str = r#"@Service
class StampService {
    private final Peers peers = new Peers();
    String connection = "";
    String channel = "";
    boolean queued;
    String description = "";
    String message = "";
    String failure = "";

    boolean available() {
        return peers.supported();
    }

    void start() {
        connection = peers.open("", "");
        peers.offer(connection);
        channel = peers.openChannel(connection, "chat", true);
    }

    void push() {
        queued = peers.send(connection, channel, "AQI");
    }

    void refuse() {
        peers.accept(connection, "bad");
    }

    @OnEvent
    void hearDescription(PeerDescription described) {
        description = described.kind() + " " + described.sdp();
    }

    @OnEvent
    void hearMessage(PeerMessage received) {
        message = received.data();
    }

    @OnEvent
    void hearFailed(PeerFailed failed) {
        failure = failed.call() + " " + failed.reason();
    }
}
"#;

    const DART: &str = r"@Service()
class StampService {
  @Needs()
  final Peers peers;
  String connection = '';
  String channel = '';
  bool queued = false;
  String description = '';
  String message = '';
  String failure = '';

  bool available() => peers.supported();

  void start() {
    connection = peers.open('', '');
    peers.offer(connection);
    channel = peers.openChannel(connection, 'chat', true);
  }

  void push() {
    queued = peers.send(connection, channel, 'AQI');
  }

  void refuse() => peers.accept(connection, 'bad');

  @OnEvent
  void hearDescription(PeerDescription described) {
    description = '${described.kind} ${described.sdp}';
  }

  @OnEvent
  void hearMessage(PeerMessage received) {
    message = received.data;
  }

  @OnEvent
  void hearFailed(PeerFailed failed) {
    failure = '${failed.call} ${failed.reason}';
  }
}
";

    const RUBY: &str = r#"#[Service]
class StampService
  attr_accessor :connection, :channel, :queued, :description, :message, :failure

  def initialize
    @connection = ""
    @channel = ""
    @queued = false
    @description = ""
    @message = ""
    @failure = ""
  end

  def available
    Peers.new.supported
  end

  def start
    @connection = Peers.new.open("", "")
    Peers.new.offer(@connection)
    @channel = Peers.new.open_channel(@connection, "chat", true)
  end

  def push
    @queued = Peers.new.send_data(@connection, @channel, "AQI")
  end

  def refuse
    Peers.new.accept(@connection, "bad")
  end

  # @param described [PeerDescription]
  #[OnEvent]
  def hear_description(described)
    @description = [described.kind, described.sdp].join(" ")
  end

  # @param received [PeerMessage]
  #[OnEvent]
  def hear_message(received)
    @message = received.data
  end

  # @param failed [PeerFailed]
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
  connection = "",
  ---@type string
  channel = "",
  ---@type boolean
  queued = false,
  ---@type string
  description = "",
  ---@type string
  message = "",
  ---@type string
  failure = "",
}

---@return boolean
function StampService:available()
  return Peers:supported()
end

---@return nil
function StampService:start()
  self.connection = Peers:open("", "")
  Peers:offer(self.connection)
  self.channel = Peers:open_channel(self.connection, "chat", true)
end

---@return nil
function StampService:push()
  self.queued = Peers:send(self.connection, self.channel, "AQI")
end

---@return nil
function StampService:refuse()
  Peers:accept(self.connection, "bad")
end

---@param described PeerDescription
---@return nil
-- @OnEvent
function StampService:hear_description(described)
  self.description = described.kind .. " " .. described.sdp
end

---@param received PeerMessage
---@return nil
-- @OnEvent
function StampService:hear_message(received)
  self.message = received.data
end

---@param failed PeerFailed
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
    [[Needs]] Peers& peers;
    std::string connection;
    std::string channel;
    bool queued = false;
    std::string description;
    std::string message;
    std::string failure;

    bool available() { return peers.supported(); }

    void start() {
        connection = peers.open("", "");
        peers.offer(connection);
        channel = peers.openChannel(connection, "chat", true);
    }

    void push() { queued = peers.send(connection, channel, "AQI"); }

    void refuse() { peers.accept(connection, "bad"); }

    [[OnEvent]] void hearDescription(const PeerDescription& described) { description = described.kind + " " + described.sdp; }

    [[OnEvent]] void hearMessage(const PeerMessage& received) { message = received.data; }

    [[OnEvent]] void hearFailed(const PeerFailed& failed) { failure = failed.call + " " + failed.reason; }
};
"#;

    fn assert_linked(root: &Path, fetches_files: bool) {
        WebApis::write(
            root,
            "application-events.mjs",
            WebApis::EVENTS_RUNTIME.as_bytes(),
        );
        WebApis::write(root, "tac-peers.js", Self::MODULE.as_bytes());
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
                "connection": "peer-1",
                "channel": "channel-1",
                "queued": true,
                "description": "offer v=0 offer",
                "message": "-_8",
                "failure": "accept invalid",
                "sent": [[1, 2]]
            })
        );
    }
}

#[tokio::test]
async fn a_rust_web_service_connects_to_a_peer() {
    if !WebApis::node_available() || !WebApis::rust_web_target() {
        eprintln!("skipping: node or the wasm32 Rust target is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Rust, "rs", PeersTests::RUST).await;
    PeersTests::assert_linked(root.path(), false);
}

#[tokio::test]
async fn a_go_web_service_connects_to_a_peer() {
    let go = Command::new("go")
        .arg("version")
        .output()
        .is_ok_and(|output| output.status.success());
    if !WebApis::node_available() || !go {
        eprintln!("skipping: node or go is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Go, "go", PeersTests::GO).await;
    PeersTests::assert_linked(root.path(), true);
}

#[tokio::test]
async fn a_csharp_web_service_connects_to_a_peer() {
    if !dotnet_wasm_ready() {
        eprintln!("skipping: node or the .NET wasm-tools workload is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = ("cs", PeersTests::CSHARP);
    let profile = crate::BuildProfile::Development;
    WebApis::publish_with(root.path(), WebLanguage::CSharp, source, profile).await;
    PeersTests::assert_linked(root.path(), false);
}

#[tokio::test]
async fn an_fsharp_web_service_connects_to_a_peer() {
    if !dotnet_wasm_ready() {
        eprintln!("skipping: node or the .NET wasm-tools workload is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = ("fs", PeersTests::FSHARP);
    let profile = crate::BuildProfile::Development;
    WebApis::publish_with(root.path(), WebLanguage::CSharp, source, profile).await;
    PeersTests::assert_linked(root.path(), false);
}

#[tokio::test]
async fn a_dart_service_connects_to_a_peer() {
    if !WebApis::node_available() || !WebApis::dart_available() {
        eprintln!("skipping: node or dart is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish_dart(root.path(), PeersTests::DART).await;
    PeersTests::assert_linked(root.path(), true);
}

#[tokio::test]
async fn a_ruby_web_service_connects_to_a_peer() {
    let unavailable = crate::tac_ruby::RubyRuntimeBuild::unavailable_reason();
    if !WebApis::node_available() || unavailable.is_some() {
        eprintln!("skipping: node or the Ruby runtime is unavailable: {unavailable:?}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Ruby, "rb", PeersTests::RUBY).await;
    PeersTests::assert_linked(root.path(), true);
}

#[tokio::test]
async fn a_lua_web_service_connects_to_a_peer() {
    if lua_unavailable() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Lua, "lua", PeersTests::LUA).await;
    PeersTests::assert_linked(root.path(), true);
}

#[tokio::test]
async fn a_swift_web_service_connects_to_a_peer() {
    let root = tempfile::tempdir().expect("project");
    if !publish_swift(root.path(), PeersTests::SWIFT).await {
        return;
    }
    PeersTests::assert_linked(root.path(), true);
}

#[tokio::test]
async fn a_kotlin_web_service_connects_to_a_peer() {
    if !super::super::kotlin_tests::kotlin_web_ready() {
        eprintln!("skipping: kotlinc-js, the Kotlin wasm stdlib or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Kotlin, "kt", PeersTests::KOTLIN).await;
    PeersTests::assert_linked(root.path(), true);
}

#[tokio::test]
async fn a_java_web_service_connects_to_a_peer() {
    let root = tempfile::tempdir().expect("project");
    if !publish_java(root.path(), PeersTests::JAVA).await {
        return;
    }
    PeersTests::assert_linked(root.path(), true);
}

#[tokio::test]
async fn a_cpp_web_service_connects_to_a_peer() {
    if !WebApis::node_available() || !WebApis::cpp_available() {
        eprintln!("skipping: node or em++ is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Cpp, "hpp", PeersTests::CPP).await;
    PeersTests::assert_linked(root.path(), true);
}
