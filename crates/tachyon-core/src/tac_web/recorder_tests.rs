use super::{
    WebApis, WebLanguage, dotnet_wasm_ready, lua_unavailable, publish_java, publish_swift,
};
use std::path::Path;
use std::process::Command;

struct RecorderTests;

impl RecorderTests {
    const SERVICE_DRIVER: &str = r";(0, eval)(`${await readFile('./tac-web.js', 'utf8')}\nglobalThis.TacWebApis = TacWebApis`)
const { default: TacApplicationEvents } = await import('./application-events.mjs')
const { default: StampService, __tachyon_event_plan__: plan } = await import('./StampService.js')
__GRANT_STUBS__
grants.content = 'files microphone'
const got = []
const later = (reply, event) => { setTimeout(() => reply(event), 0); return null }
const heard = (call) => (reply, ...values) => { got.push([call, ...values]); return null }
globalThis[Symbol.for('tachyon.tac.recorder')] = Object.freeze({
  supported: () => true,
  grant: () => 'microphone',
  calls: ['start', 'addAudio', 'removeAudio', 'showVideo', 'pause', 'resume', 'stop'],
  start: (reply, recording, audioTracks, videoTrack, fileName, format) => { got.push(['start', recording, audioTracks, videoTrack, fileName, format]); return later(reply, ['RecordingStarted', { recording, format: 'webm', fileName: `${fileName}.webm` }]) },
  addAudio: heard('addAudio'),
  removeAudio: heard('removeAudio'),
  showVideo: heard('showVideo'),
  pause: heard('pause'),
  resume: heard('resume'),
  stop: (reply, recording) => { got.push(['stop', recording]); return later(reply, ['RecordingStopped', { recording, bytes: 2048, seconds: 3 }]) },
})
const settle = async () => { for (let round = 0; round < 10; round += 1) await new Promise((resolve) => setTimeout(resolve, 0)) }
const bus = new TacApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })
TacWebApis.events = { publish: (name, value) => bus.publish(name, value), send: (owner, name, value) => bus.send(owner, name, value) }
const stamp = new StampService({})
bus.attach(stamp, plan.types[0], { applicationEventPublisher: null })
await bus.initialize(stamp)
const ready = stamp.available()
stamp.capture()
await settle()
stamp.finish()
await settle()
console.log(JSON.stringify({ ready, began: stamp.began, ended: stamp.ended, got }))
process.exit(0)
";

    const RUST: &str = r#"#[Service]
#[derive(Default)]
struct StampService {
    recorder: Recorder,
    began: String,
    ended: String,
}

impl StampService {
    pub fn available(&self) -> bool {
        self.recorder.supported().unwrap_or(false)
    }

    pub fn capture(&self) {
        let _ = self.recorder.start("meeting", "mic", "cam", "Meeting", "");
        let _ = self.recorder.show_video("meeting", "screen");
        let _ = self.recorder.pause("meeting");
    }

    pub fn finish(&self) {
        let _ = self.recorder.stop("meeting");
    }

    #[OnEvent]
    pub fn hear_started(&mut self, started: RecordingStarted) {
        self.began = format!("{} {} {}", started.recording, started.format, started.fileName);
    }

    #[OnEvent]
    pub fn hear_stopped(&mut self, stopped: RecordingStopped) {
        self.ended = stopped.recording;
    }
}
"#;

    const GO: &str = r#"// @Service
type StampService struct {
	// @Needs
	recorder Recorder
	Began    string
	Ended    string
}

func (stamp *StampService) Available() bool {
	supported, _ := stamp.recorder.Supported()
	return supported
}

func (stamp *StampService) Capture() {
	_ = stamp.recorder.Start("meeting", "mic", "cam", "Meeting", "")
	_ = stamp.recorder.ShowVideo("meeting", "screen")
	_ = stamp.recorder.Pause("meeting")
}

func (stamp *StampService) Finish() {
	_ = stamp.recorder.Stop("meeting")
}

// @OnEvent
func (stamp *StampService) HearStarted(started RecordingStarted) {
	stamp.Began = started.Recording + " " + started.Format + " " + started.FileName
}

// @OnEvent
func (stamp *StampService) HearStopped(stopped RecordingStopped) {
	stamp.Ended = stopped.Recording
}
"#;

    const CSHARP: &str = r#"[Service]
public class StampService
{
    private readonly Recorder recorder = new();
    public string Began = "";
    public string Ended = "";

    public bool Available() => recorder.Supported();

    public void Capture()
    {
        recorder.Start("meeting", "mic", "cam", "Meeting", "");
        recorder.ShowVideo("meeting", "screen");
        recorder.Pause("meeting");
    }

    public void Finish() => recorder.Stop("meeting");

    [OnEvent]
    public void HearStarted(RecordingStarted started)
    {
        Began = started.Recording + " " + started.Format + " " + started.FileName;
    }

    [OnEvent]
    public void HearStopped(RecordingStopped stopped)
    {
        Ended = stopped.Recording;
    }
}
"#;

    const FSHARP: &str = r#"[<Service>]
type StampService() =
    let recorder = Recorder()
    member val Began: string = "" with get, set
    member val Ended: string = "" with get, set
    member this.Available() : bool = recorder.Supported()
    member this.Capture() : unit =
        recorder.Start("meeting", "mic", "cam", "Meeting", "")
        recorder.ShowVideo("meeting", "screen")
        recorder.Pause("meeting")
    member this.Finish() : unit = recorder.Stop("meeting")
    [<OnEvent>]
    member this.HearStarted(started: RecordingStarted) : unit = this.Began <- started.Recording + " " + started.Format + " " + started.FileName
    [<OnEvent>]
    member this.HearStopped(stopped: RecordingStopped) : unit = this.Ended <- stopped.Recording
"#;

    const SWIFT: &str = r#"@Service
final class StampService {
    private let recorder = Recorder()
    var began: String = ""
    var ended: String = ""

    func available() -> Bool {
        (try? recorder.supported()) ?? false
    }

    func capture() throws {
        try recorder.start(recording: "meeting", audioTracks: "mic", videoTrack: "cam", fileName: "Meeting", format: "")
        try recorder.showVideo(recording: "meeting", track: "screen")
        try recorder.pause(recording: "meeting")
    }

    func finish() throws {
        try recorder.stop(recording: "meeting")
    }

    @OnEvent
    func hearStarted(_ started: RecordingStarted) {
        began = "\(started.recording) \(started.format) \(started.fileName)"
    }

    @OnEvent
    func hearStopped(_ stopped: RecordingStopped) {
        ended = stopped.recording
    }
}
"#;

    const KOTLIN: &str = r#"@Service
class StampService {
    private val recorder = Recorder()
    var began: String = ""
    var ended: String = ""

    fun available(): Boolean = recorder.supported()

    fun capture() {
        recorder.start("meeting", "mic", "cam", "Meeting", "")
        recorder.showVideo("meeting", "screen")
        recorder.pause("meeting")
    }

    fun finish() {
        recorder.stop("meeting")
    }

    @OnEvent
    fun hearStarted(started: RecordingStarted) {
        began = started.recording + " " + started.format + " " + started.fileName
    }

    @OnEvent
    fun hearStopped(stopped: RecordingStopped) {
        ended = stopped.recording
    }
}
"#;

    const JAVA: &str = r#"@Service
class StampService {
    private final Recorder recorder = new Recorder();
    String began = "";
    String ended = "";

    boolean available() {
        return recorder.supported();
    }

    void capture() {
        recorder.start("meeting", "mic", "cam", "Meeting", "");
        recorder.showVideo("meeting", "screen");
        recorder.pause("meeting");
    }

    void finish() {
        recorder.stop("meeting");
    }

    @OnEvent
    void hearStarted(RecordingStarted started) {
        began = started.recording() + " " + started.format() + " " + started.fileName();
    }

    @OnEvent
    void hearStopped(RecordingStopped stopped) {
        ended = stopped.recording();
    }
}
"#;

    const DART: &str = r"@Service()
class StampService {
  @Needs()
  final Recorder recorder;
  String began = '';
  String ended = '';

  bool available() => recorder.supported();

  void capture() {
    recorder.start('meeting', 'mic', 'cam', 'Meeting', '');
    recorder.showVideo('meeting', 'screen');
    recorder.pause('meeting');
  }

  void finish() => recorder.stop('meeting');

  @OnEvent
  void hearStarted(RecordingStarted started) {
    began = '${started.recording} ${started.format} ${started.fileName}';
  }

  @OnEvent
  void hearStopped(RecordingStopped stopped) {
    ended = stopped.recording;
  }
}
";

    const RUBY: &str = r#"#[Service]
class StampService
  attr_accessor :began, :ended

  def initialize
    @began = ""
    @ended = ""
  end

  def available
    Recorder.new.supported
  end

  def capture
    Recorder.new.start("meeting", "mic", "cam", "Meeting", "")
    Recorder.new.show_video("meeting", "screen")
    Recorder.new.pause("meeting")
  end

  def finish
    Recorder.new.stop("meeting")
  end

  # @param started [RecordingStarted]
  #[OnEvent]
  def hear_started(started)
    @began = [started.recording, started.format, started.fileName].join(" ")
  end

  # @param stopped [RecordingStopped]
  #[OnEvent]
  def hear_stopped(stopped)
    @ended = stopped.recording
  end
end
"#;

    const LUA: &str = r#"-- @Service
---@class StampService
StampService = {
  ---@type string
  began = "",
  ---@type string
  ended = "",
}

---@return boolean
function StampService:available()
  return Recorder:supported()
end

---@return nil
function StampService:capture()
  Recorder:start("meeting", "mic", "cam", "Meeting", "")
  Recorder:show_video("meeting", "screen")
  Recorder:pause("meeting")
end

---@return nil
function StampService:finish()
  Recorder:stop("meeting")
end

---@param started RecordingStarted
---@return nil
-- @OnEvent
function StampService:hear_started(started)
  self.began = started.recording .. " " .. started.format .. " " .. started.fileName
end

---@param stopped RecordingStopped
---@return nil
-- @OnEvent
function StampService:hear_stopped(stopped)
  self.ended = stopped.recording
end
"#;

    const CPP: &str = r#"#pragma once
#include <string>

class [[Service]] StampService {
public:
    [[Needs]] Recorder& recorder;
    std::string began;
    std::string ended;

    bool available() { return recorder.supported(); }

    void capture() {
        recorder.start("meeting", "mic", "cam", "Meeting", "");
        recorder.showVideo("meeting", "screen");
        recorder.pause("meeting");
    }

    void finish() { recorder.stop("meeting"); }

    [[OnEvent]] void hearStarted(const RecordingStarted& started) { began = started.recording + " " + started.format + " " + started.fileName; }

    [[OnEvent]] void hearStopped(const RecordingStopped& stopped) { ended = stopped.recording; }
};
"#;

    fn assert_recorded(root: &Path, fetches_files: bool) {
        WebApis::write(
            root,
            "application-events.mjs",
            WebApis::EVENTS_RUNTIME.as_bytes(),
        );
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
                "began": "meeting webm Meeting.webm",
                "ended": "meeting",
                "got": [
                    ["start", "meeting", "mic", "cam", "Meeting", ""],
                    ["showVideo", "meeting", "screen"],
                    ["pause", "meeting"],
                    ["stop", "meeting"]
                ]
            })
        );
    }
}

#[tokio::test]
async fn a_rust_web_service_records_tracks() {
    if !WebApis::node_available() || !WebApis::rust_web_target() {
        eprintln!("skipping: node or the wasm32 Rust target is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Rust, "rs", RecorderTests::RUST).await;
    RecorderTests::assert_recorded(root.path(), false);
}

#[tokio::test]
async fn a_go_web_service_records_tracks() {
    let go = Command::new("go")
        .arg("version")
        .output()
        .is_ok_and(|output| output.status.success());
    if !WebApis::node_available() || !go {
        eprintln!("skipping: node or go is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Go, "go", RecorderTests::GO).await;
    RecorderTests::assert_recorded(root.path(), true);
}

#[tokio::test]
async fn a_csharp_web_service_records_tracks() {
    if !dotnet_wasm_ready() {
        eprintln!("skipping: node or the .NET wasm-tools workload is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = ("cs", RecorderTests::CSHARP);
    let profile = crate::BuildProfile::Development;
    WebApis::publish_with(root.path(), WebLanguage::CSharp, source, profile).await;
    RecorderTests::assert_recorded(root.path(), false);
}

#[tokio::test]
async fn an_fsharp_web_service_records_tracks() {
    if !dotnet_wasm_ready() {
        eprintln!("skipping: node or the .NET wasm-tools workload is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = ("fs", RecorderTests::FSHARP);
    let profile = crate::BuildProfile::Development;
    WebApis::publish_with(root.path(), WebLanguage::CSharp, source, profile).await;
    RecorderTests::assert_recorded(root.path(), false);
}

#[tokio::test]
async fn a_dart_service_records_tracks() {
    if !WebApis::node_available() || !WebApis::dart_available() {
        eprintln!("skipping: node or dart is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish_dart(root.path(), RecorderTests::DART).await;
    RecorderTests::assert_recorded(root.path(), true);
}

#[tokio::test]
async fn a_ruby_web_service_records_tracks() {
    let unavailable = crate::tac_ruby::RubyRuntimeBuild::unavailable_reason();
    if !WebApis::node_available() || unavailable.is_some() {
        eprintln!("skipping: node or the Ruby runtime is unavailable: {unavailable:?}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Ruby, "rb", RecorderTests::RUBY).await;
    RecorderTests::assert_recorded(root.path(), true);
}

#[tokio::test]
async fn a_lua_web_service_records_tracks() {
    if lua_unavailable() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Lua, "lua", RecorderTests::LUA).await;
    RecorderTests::assert_recorded(root.path(), true);
}

#[tokio::test]
async fn a_swift_web_service_records_tracks() {
    let root = tempfile::tempdir().expect("project");
    if !publish_swift(root.path(), RecorderTests::SWIFT).await {
        return;
    }
    RecorderTests::assert_recorded(root.path(), true);
}

#[tokio::test]
async fn a_kotlin_web_service_records_tracks() {
    if !super::super::kotlin_tests::kotlin_web_ready() {
        eprintln!("skipping: kotlinc-js, the Kotlin wasm stdlib or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(
        root.path(),
        WebLanguage::Kotlin,
        "kt",
        RecorderTests::KOTLIN,
    )
    .await;
    RecorderTests::assert_recorded(root.path(), true);
}

#[tokio::test]
async fn a_java_web_service_records_tracks() {
    let root = tempfile::tempdir().expect("project");
    if !publish_java(root.path(), RecorderTests::JAVA).await {
        return;
    }
    RecorderTests::assert_recorded(root.path(), true);
}

#[tokio::test]
async fn a_cpp_web_service_records_tracks() {
    if !WebApis::node_available() || !WebApis::cpp_available() {
        eprintln!("skipping: node or em++ is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Cpp, "hpp", RecorderTests::CPP).await;
    RecorderTests::assert_recorded(root.path(), true);
}
