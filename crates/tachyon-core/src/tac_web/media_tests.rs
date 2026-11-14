use super::{
    WebApis, WebLanguage, dotnet_wasm_ready, lua_unavailable, publish_java, publish_swift,
};
use std::path::Path;
use std::process::Command;

struct MediaTests;

impl MediaTests {
    const MODULE: &str = include_str!("../template/tac-media.js");

    const SERVICE_DRIVER: &str = r";(0, eval)(`${await readFile('./tac-web.js', 'utf8')}\nglobalThis.TacWebApis = TacWebApis`)
;(0, eval)(await readFile('./tac-media.js', 'utf8'))
const { default: TacApplicationEvents } = await import('./application-events.mjs')
const { default: StampService, __tachyon_event_plan__: plan } = await import('./StampService.js')
__GRANT_STUBS__
grants.content = 'microphone'
globalThis.addEventListener = () => {}
const asked = []
const microphone = { kind: 'audio', label: 'Built-in', enabled: true, getSettings: () => ({ deviceId: 'mic-1' }), addEventListener: () => {}, stop: () => {} }
navigator.mediaDevices = {
  getUserMedia: async (constraints) => { asked.push(constraints); return { getTracks: () => [microphone] } },
  enumerateDevices: async () => [{ kind: 'audioinput', deviceId: 'mic-1', label: 'Built-in' }],
  addEventListener: () => {},
}
const settle = async () => { for (let round = 0; round < 10; round += 1) await new Promise((resolve) => setTimeout(resolve, 0)) }
const bus = new TacApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })
TacWebApis.events = { publish: (name, value) => bus.publish(name, value), send: (owner, name, value) => bus.send(owner, name, value) }
const stamp = new StampService({})
bus.attach(stamp, plan.types[0], { applicationEventPublisher: null })
await bus.initialize(stamp)
const ready = stamp.available()
stamp.start()
await settle()
try { stamp.refuse() } catch {}
await settle()
console.log(JSON.stringify({ ready, started: stamp.started, failure: stamp.failure, asked }))
process.exit(0)
";

    const RUST: &str = r#"#[Service]
#[derive(Default)]
struct StampService {
    media: Media,
    started: String,
    failure: String,
}

impl StampService {
    pub fn available(&self) -> bool {
        self.media.supported("capture").unwrap_or(false)
    }

    pub fn start(&self) {
        let _ = self.media.capture_microphone("mic", "", true);
    }

    pub fn refuse(&self) {
        let _ = self.media.capture_camera("cam", "", "low");
    }

    #[OnEvent]
    pub fn hear_started(&mut self, track: MediaTrackStarted) {
        self.started = format!("{} {} {}", track.track, track.kind, track.label);
    }

    #[OnEvent]
    pub fn hear_failed(&mut self, failed: MediaFailed) {
        self.failure = format!("{} {}", failed.call, failed.reason);
    }
}
"#;

    const GO: &str = r#"// @Service
type StampService struct {
	// @Needs
	media   Media
	Started string
	Failure string
}

func (stamp *StampService) Available() bool {
	supported, _ := stamp.media.Supported("capture")
	return supported
}

func (stamp *StampService) Start() {
	_ = stamp.media.CaptureMicrophone("mic", "", true)
}

func (stamp *StampService) Refuse() {
	_ = stamp.media.CaptureCamera("cam", "", "low")
}

// @OnEvent
func (stamp *StampService) HearStarted(track MediaTrackStarted) {
	stamp.Started = track.Track + " " + track.Kind + " " + track.Label
}

// @OnEvent
func (stamp *StampService) HearFailed(failed MediaFailed) {
	stamp.Failure = failed.Call + " " + failed.Reason
}
"#;

    const CSHARP: &str = r#"[Service]
public class StampService
{
    private readonly Media media = new();
    public string Started = "";
    public string Failure = "";

    public bool Available() => media.Supported("capture");

    public void Start() => media.CaptureMicrophone("mic", "", true);

    public void Refuse() => media.CaptureCamera("cam", "", "low");

    [OnEvent]
    public void HearStarted(MediaTrackStarted track)
    {
        Started = track.Track + " " + track.Kind + " " + track.Label;
    }

    [OnEvent]
    public void HearFailed(MediaFailed failed)
    {
        Failure = failed.Call + " " + failed.Reason;
    }
}
"#;

    const FSHARP: &str = r#"[<Service>]
type StampService() =
    let media = Media()
    member val Started: string = "" with get, set
    member val Failure: string = "" with get, set
    member this.Available() : bool = media.Supported("capture")
    member this.Start() : unit = media.CaptureMicrophone("mic", "", true)
    member this.Refuse() : unit = media.CaptureCamera("cam", "", "low")
    [<OnEvent>]
    member this.HearStarted(track: MediaTrackStarted) : unit = this.Started <- track.Track + " " + track.Kind + " " + track.Label
    [<OnEvent>]
    member this.HearFailed(failed: MediaFailed) : unit = this.Failure <- failed.Call + " " + failed.Reason
"#;

    const SWIFT: &str = r#"@Service
final class StampService {
    private let media = Media()
    var started: String = ""
    var failure: String = ""

    func available() -> Bool {
        (try? media.supported(feature: "capture")) ?? false
    }

    func start() throws {
        try media.captureMicrophone(track: "mic", device: "", processing: true)
    }

    func refuse() {
        try? media.captureCamera(track: "cam", device: "", quality: "low")
    }

    @OnEvent
    func hearStarted(_ track: MediaTrackStarted) {
        started = "\(track.track) \(track.kind) \(track.label)"
    }

    @OnEvent
    func hearFailed(_ failed: MediaFailed) {
        failure = "\(failed.call) \(failed.reason)"
    }
}
"#;

    const KOTLIN: &str = r#"@Service
class StampService {
    private val media = Media()
    var started: String = ""
    var failure: String = ""

    fun available(): Boolean = media.supported("capture")

    fun start() {
        media.captureMicrophone("mic", "", true)
    }

    fun refuse() {
        media.captureCamera("cam", "", "low")
    }

    @OnEvent
    fun hearStarted(track: MediaTrackStarted) {
        started = track.track + " " + track.kind + " " + track.label
    }

    @OnEvent
    fun hearFailed(failed: MediaFailed) {
        failure = failed.call + " " + failed.reason
    }
}
"#;

    const JAVA: &str = r#"@Service
class StampService {
    private final Media media = new Media();
    String started = "";
    String failure = "";

    boolean available() {
        return media.supported("capture");
    }

    void start() {
        media.captureMicrophone("mic", "", true);
    }

    void refuse() {
        media.captureCamera("cam", "", "low");
    }

    @OnEvent
    void hearStarted(MediaTrackStarted track) {
        started = track.track() + " " + track.kind() + " " + track.label();
    }

    @OnEvent
    void hearFailed(MediaFailed failed) {
        failure = failed.call() + " " + failed.reason();
    }
}
"#;

    const DART: &str = r"@Service()
class StampService {
  @Needs()
  final Media media;
  String started = '';
  String failure = '';

  bool available() => media.supported('capture');

  void start() => media.captureMicrophone('mic', '', true);

  void refuse() => media.captureCamera('cam', '', 'low');

  @OnEvent
  void hearStarted(MediaTrackStarted track) {
    started = '${track.track} ${track.kind} ${track.label}';
  }

  @OnEvent
  void hearFailed(MediaFailed failed) {
    failure = '${failed.call} ${failed.reason}';
  }
}
";

    const RUBY: &str = r#"#[Service]
class StampService
  attr_accessor :started, :failure

  def initialize
    @started = ""
    @failure = ""
  end

  def available
    Media.new.supported("capture")
  end

  def start
    Media.new.capture_microphone("mic", "", true)
  end

  def refuse
    Media.new.capture_camera("cam", "", "low")
  rescue RuntimeError
    nil
  end

  # @param track [MediaTrackStarted]
  #[OnEvent]
  def hear_started(track)
    @started = [track.track, track.kind, track.label].join(" ")
  end

  # @param failed [MediaFailed]
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
  started = "",
  ---@type string
  failure = "",
}

---@return boolean
function StampService:available()
  return Media:supported("capture")
end

---@return nil
function StampService:start()
  Media:capture_microphone("mic", "", true)
end

---@return nil
function StampService:refuse()
  pcall(Media.capture_camera, Media, "cam", "", "low")
end

---@param track MediaTrackStarted
---@return nil
-- @OnEvent
function StampService:hear_started(track)
  self.started = track.track .. " " .. track.kind .. " " .. track.label
end

---@param failed MediaFailed
---@return nil
-- @OnEvent
function StampService:hear_failed(failed)
  self.failure = failed.call .. " " .. failed.reason
end
"#;

    const CPP: &str = r#"#pragma once
#include <stdexcept>
#include <string>

class [[Service]] StampService {
public:
    [[Needs]] Media& media;
    std::string started;
    std::string failure;

    bool available() { return media.supported("capture"); }

    void start() { media.captureMicrophone("mic", "", true); }

    void refuse() {
        try {
            media.captureCamera("cam", "", "low");
        } catch (const std::runtime_error&) {
        }
    }

    [[OnEvent]] void hearStarted(const MediaTrackStarted& track) { started = track.track + " " + track.kind + " " + track.label; }

    [[OnEvent]] void hearFailed(const MediaFailed& failed) { failure = failed.call + " " + failed.reason; }
};
"#;

    fn assert_captured(root: &Path, fetches_files: bool) {
        WebApis::write(
            root,
            "application-events.mjs",
            WebApis::EVENTS_RUNTIME.as_bytes(),
        );
        WebApis::write(root, "tac-media.js", Self::MODULE.as_bytes());
        let captured = Self::SERVICE_DRIVER.replace("__GRANT_STUBS__", WebApis::GRANT_STUBS);
        let driver = if fetches_files {
            format!("{}{captured}", WebApis::FILE_FETCH)
        } else {
            captured
        };
        let report = WebApis::run(root, &driver);
        assert_eq!(
            report,
            serde_json::json!({
                "ready": true,
                "started": "mic audio Built-in",
                "failure": "",
                "asked": [{
                    "audio": {
                        "echoCancellation": true,
                        "noiseSuppression": true,
                        "autoGainControl": true
                    }
                }]
            })
        );
    }
}

#[tokio::test]
async fn a_rust_web_service_captures_media_and_hears_media_events() {
    if !WebApis::node_available() || !WebApis::rust_web_target() {
        eprintln!("skipping: node or the wasm32 Rust target is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Rust, "rs", MediaTests::RUST).await;
    MediaTests::assert_captured(root.path(), false);
}

#[tokio::test]
async fn a_go_web_service_captures_media_and_hears_media_events() {
    let go = Command::new("go")
        .arg("version")
        .output()
        .is_ok_and(|output| output.status.success());
    if !WebApis::node_available() || !go {
        eprintln!("skipping: node or go is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Go, "go", MediaTests::GO).await;
    MediaTests::assert_captured(root.path(), true);
}

#[tokio::test]
async fn a_csharp_web_service_captures_media_and_hears_media_events() {
    if !dotnet_wasm_ready() {
        eprintln!("skipping: node or the .NET wasm-tools workload is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = ("cs", MediaTests::CSHARP);
    let profile = crate::BuildProfile::Development;
    WebApis::publish_with(root.path(), WebLanguage::CSharp, source, profile).await;
    MediaTests::assert_captured(root.path(), false);
}

#[tokio::test]
async fn an_fsharp_web_service_captures_media_and_hears_media_events() {
    if !dotnet_wasm_ready() {
        eprintln!("skipping: node or the .NET wasm-tools workload is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = ("fs", MediaTests::FSHARP);
    let profile = crate::BuildProfile::Development;
    WebApis::publish_with(root.path(), WebLanguage::CSharp, source, profile).await;
    MediaTests::assert_captured(root.path(), false);
}

#[tokio::test]
async fn a_dart_service_captures_media_and_hears_media_events() {
    if !WebApis::node_available() || !WebApis::dart_available() {
        eprintln!("skipping: node or dart is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish_dart(root.path(), MediaTests::DART).await;
    MediaTests::assert_captured(root.path(), true);
}

#[tokio::test]
async fn a_ruby_web_service_captures_media_and_hears_media_events() {
    let unavailable = crate::tac_ruby::RubyRuntimeBuild::unavailable_reason();
    if !WebApis::node_available() || unavailable.is_some() {
        eprintln!("skipping: node or the Ruby runtime is unavailable: {unavailable:?}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Ruby, "rb", MediaTests::RUBY).await;
    MediaTests::assert_captured(root.path(), true);
}

#[tokio::test]
async fn a_lua_web_service_captures_media_and_hears_media_events() {
    if lua_unavailable() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Lua, "lua", MediaTests::LUA).await;
    MediaTests::assert_captured(root.path(), true);
}

#[tokio::test]
async fn a_swift_web_service_captures_media_and_hears_media_events() {
    let root = tempfile::tempdir().expect("project");
    if !publish_swift(root.path(), MediaTests::SWIFT).await {
        return;
    }
    MediaTests::assert_captured(root.path(), true);
}

#[tokio::test]
async fn a_kotlin_web_service_captures_media_and_hears_media_events() {
    if !super::super::kotlin_tests::kotlin_web_ready() {
        eprintln!("skipping: kotlinc-js, the Kotlin wasm stdlib or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Kotlin, "kt", MediaTests::KOTLIN).await;
    MediaTests::assert_captured(root.path(), true);
}

#[tokio::test]
async fn a_java_web_service_captures_media_and_hears_media_events() {
    let root = tempfile::tempdir().expect("project");
    if !publish_java(root.path(), MediaTests::JAVA).await {
        return;
    }
    MediaTests::assert_captured(root.path(), true);
}

#[tokio::test]
async fn a_cpp_web_service_captures_media_and_hears_media_events() {
    if !WebApis::node_available() || !WebApis::cpp_available() {
        eprintln!("skipping: node or em++ is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Cpp, "hpp", MediaTests::CPP).await;
    MediaTests::assert_captured(root.path(), true);
}
