use super::{
    WebApis, WebLanguage, dotnet_wasm_ready, lua_unavailable, publish_java, publish_swift,
};
use std::path::Path;
use std::process::Command;

struct BlobsTests;

impl BlobsTests {
    const SERVICE_DRIVER: &str = r";(0, eval)(`${await readFile('./tac-web.js', 'utf8')}\nglobalThis.TacWebApis = TacWebApis`)
const { default: TacApplicationEvents } = await import('./application-events.mjs')
const { default: StampService, __tachyon_event_plan__: plan } = await import('./StampService.js')
__GRANT_STUBS__
grants.content = 'blobs'
const asked = []
const later = (reply, event) => setTimeout(() => reply(event), 0)
globalThis[Symbol.for('tachyon.tac.blobs')] = {
  calls: ['saveHeld', 'read', 'url', 'upload'],
  saveHeld: (reply, id, key) => { asked.push(['saveHeld', id, key]); later(reply, ['BlobSaved', { key, bytes: 5, media: 'text/plain', sha256: `sha-${id}` }]); return null },
  read: (reply, key, offset, count) => { asked.push(['read', key, offset, count]); later(reply, ['BlobChunkRead', { key, offset, size: 5, data: 'aGVsbG8=' }]); return null },
  url: (reply, key) => `blob:${key}`,
  upload: (reply, key, method, url, headers) => { asked.push(['upload', key, method, url, headers]); return `${method}-1` },
}
const bus = new TacApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })
TacWebApis.events = { publish: (name, value) => bus.publish(name, value), send: (owner, name, value) => bus.send(owner, name, value) }
const attachment = new StampService({})
bus.attach(attachment, plan.types[0], { applicationEventPublisher: null })
await bus.initialize(attachment)
attachment.keep()
attachment.look()
attachment.send()
for (let round = 0; round < 20; round += 1) await new Promise((resolve) => setTimeout(resolve, 0))
console.log(JSON.stringify({ bytes: attachment.bytes, sha: attachment.sha, chunk: attachment.chunk, shown: attachment.shown, upload: attachment.upload, asked }))
process.exit(0)
";

    const RUST: &str = r#"#[Service]
#[derive(Default)]
struct StampService {
    blobs: Blobs,
    bytes: f64,
    sha: String,
    chunk: String,
    shown: String,
    upload: String,
}

impl StampService {
    pub fn keep(&self) {
        let _ = self.blobs.save_held("file-1", "notes/a");
    }

    pub fn look(&mut self) {
        let _ = self.blobs.read("notes/a", 0, 5);
        self.shown = self.blobs.url("notes/a").unwrap_or_default();
    }

    pub fn send(&mut self) {
        self.upload = self.blobs.upload("notes/a", "PUT", "/uploads", "").unwrap_or_default();
    }

    #[OnEvent]
    pub fn hear_saved(&mut self, saved: BlobSaved) {
        self.bytes = saved.bytes;
        self.sha = saved.sha256;
    }

    #[OnEvent]
    pub fn hear_chunk(&mut self, read: BlobChunkRead) {
        self.chunk = read.data;
    }
}
"#;

    const GO: &str = r#"// @Service
type StampService struct {
	// @Needs
	blobs  Blobs
	Bytes  float64
	Sha    string
	Chunk  string
	Shown  string
	Upload string
}

func (attachment *StampService) Keep() {
	_ = attachment.blobs.SaveHeld("file-1", "notes/a")
}

func (attachment *StampService) Look() {
	_ = attachment.blobs.Read("notes/a", 0, 5)
	attachment.Shown, _ = attachment.blobs.Url("notes/a")
}

func (attachment *StampService) Send() {
	attachment.Upload, _ = attachment.blobs.Upload("notes/a", "PUT", "/uploads", "")
}

// @OnEvent
func (attachment *StampService) HearSaved(saved BlobSaved) {
	attachment.Bytes = saved.Bytes
	attachment.Sha = saved.Sha256
}

// @OnEvent
func (attachment *StampService) HearChunk(read BlobChunkRead) {
	attachment.Chunk = read.Data
}
"#;

    const CSHARP: &str = r#"[Service]
public class StampService
{
    private readonly Blobs blobs = new();
    public double Bytes = 0;
    public string Sha = "";
    public string Chunk = "";
    public string Shown = "";
    public string Upload = "";

    public void Keep() => blobs.SaveHeld("file-1", "notes/a");

    public void Look()
    {
        blobs.Read("notes/a", 0, 5);
        Shown = blobs.Url("notes/a");
    }

    public void Send() => Upload = blobs.Upload("notes/a", "PUT", "/uploads", "");

    [OnEvent]
    public void HearSaved(BlobSaved saved)
    {
        Bytes = saved.Bytes;
        Sha = saved.Sha256;
    }

    [OnEvent]
    public void HearChunk(BlobChunkRead read)
    {
        Chunk = read.Data;
    }
}
"#;

    const FSHARP: &str = r#"[<Service>]
type StampService() =
    let blobs = Blobs()
    member val Bytes: float = 0.0 with get, set
    member val Sha: string = "" with get, set
    member val Chunk: string = "" with get, set
    member val Shown: string = "" with get, set
    member val Upload: string = "" with get, set
    member this.Keep() : unit = blobs.SaveHeld("file-1", "notes/a")
    member this.Look() : unit =
        blobs.Read("notes/a", 0L, 5)
        this.Shown <- blobs.Url("notes/a")
    member this.Send() : unit = this.Upload <- blobs.Upload("notes/a", "PUT", "/uploads", "")
    [<OnEvent>]
    member this.HearSaved(saved: BlobSaved) : unit =
        this.Bytes <- saved.Bytes
        this.Sha <- saved.Sha256
    [<OnEvent>]
    member this.HearChunk(read: BlobChunkRead) : unit = this.Chunk <- read.Data
"#;

    const SWIFT: &str = r#"@Service
final class StampService {
    private let blobs = Blobs()
    var bytes: Double = 0
    var sha: String = ""
    var chunk: String = ""
    var shown: String = ""
    var upload: String = ""

    func keep() throws {
        try blobs.saveHeld(fileId: "file-1", key: "notes/a")
    }

    func look() throws {
        try blobs.read(key: "notes/a", offset: 0, count: 5)
        shown = try blobs.url(key: "notes/a")
    }

    func send() throws {
        upload = try blobs.upload(key: "notes/a", method: "PUT", url: "/uploads", headers: "")
    }

    @OnEvent
    func hearSaved(_ saved: BlobSaved) {
        bytes = saved.bytes
        sha = saved.sha256
    }

    @OnEvent
    func hearChunk(_ read: BlobChunkRead) {
        chunk = read.data
    }
}
"#;

    const KOTLIN: &str = r#"@Service
class StampService {
    private val blobs = Blobs()
    var bytes: Double = 0.0
    var sha: String = ""
    var chunk: String = ""
    var shown: String = ""
    var upload: String = ""

    fun keep() {
        blobs.saveHeld("file-1", "notes/a")
    }

    fun look() {
        blobs.read("notes/a", 0L, 5)
        shown = blobs.url("notes/a")
    }

    fun send() {
        upload = blobs.upload("notes/a", "PUT", "/uploads", "")
    }

    @OnEvent
    fun hearSaved(saved: BlobSaved) {
        bytes = saved.bytes
        sha = saved.sha256
    }

    @OnEvent
    fun hearChunk(read: BlobChunkRead) {
        chunk = read.data
    }
}
"#;

    const JAVA: &str = r#"@Service
class StampService {
    private final Blobs blobs = new Blobs();
    double bytes = 0;
    String sha = "";
    String chunk = "";
    String shown = "";
    String upload = "";

    void keep() {
        blobs.saveHeld("file-1", "notes/a");
    }

    void look() {
        blobs.read("notes/a", 0, 5);
        shown = blobs.url("notes/a");
    }

    void send() {
        upload = blobs.upload("notes/a", "PUT", "/uploads", "");
    }

    @OnEvent
    void hearSaved(BlobSaved saved) {
        bytes = saved.bytes();
        sha = saved.sha256();
    }

    @OnEvent
    void hearChunk(BlobChunkRead read) {
        chunk = read.data();
    }
}
"#;

    const DART: &str = r"@Service()
class StampService {
  @Needs()
  final Blobs blobs;
  double bytes = 0;
  String sha = '';
  String chunk = '';
  String shown = '';
  String upload = '';

  void keep() => blobs.saveHeld('file-1', 'notes/a');

  void look() {
    blobs.read('notes/a', 0, 5);
    shown = blobs.url('notes/a');
  }

  void send() {
    upload = blobs.upload('notes/a', 'PUT', '/uploads', '');
  }

  @OnEvent
  void hearSaved(BlobSaved saved) {
    bytes = saved.bytes;
    sha = saved.sha256;
  }

  @OnEvent
  void hearChunk(BlobChunkRead read) {
    chunk = read.data;
  }
}
";

    const RUBY: &str = r#"#[Service]
class StampService
  attr_accessor :bytes, :sha, :chunk, :shown, :upload

  def initialize
    @bytes = 0.0
    @sha = ""
    @chunk = ""
    @shown = ""
    @upload = ""
  end

  def keep
    Blobs.new.save_held("file-1", "notes/a")
  end

  def look
    Blobs.new.read("notes/a", 0, 5)
    @shown = Blobs.new.url("notes/a")
  end

  def send
    @upload = Blobs.new.upload("notes/a", "PUT", "/uploads", "")
  end

  # @param saved [BlobSaved]
  #[OnEvent]
  def hear_saved(saved)
    @bytes = saved.bytes
    @sha = saved.sha256
  end

  # @param read [BlobChunkRead]
  #[OnEvent]
  def hear_chunk(read)
    @chunk = read.data
  end
end
"#;

    const LUA: &str = r#"-- @Service
---@class StampService
StampService = {
  ---@type number
  bytes = 0.0,
  ---@type string
  sha = "",
  ---@type string
  chunk = "",
  ---@type string
  shown = "",
  ---@type string
  upload = "",
}

---@return nil
function StampService:keep()
  Blobs:save_held("file-1", "notes/a")
end

---@return nil
function StampService:look()
  Blobs:read("notes/a", 0, 5)
  self.shown = Blobs:url("notes/a")
end

---@return nil
function StampService:send()
  self.upload = Blobs:upload("notes/a", "PUT", "/uploads", "")
end

---@param saved BlobSaved
---@return nil
-- @OnEvent
function StampService:hear_saved(saved)
  self.bytes = saved.bytes
  self.sha = saved.sha256
end

---@param read BlobChunkRead
---@return nil
-- @OnEvent
function StampService:hear_chunk(read)
  self.chunk = read.data
end
"#;

    const CPP: &str = r#"#pragma once
#include <string>

class [[Service]] StampService {
public:
    [[Needs]] Blobs& blobs;
    double bytes = 0;
    std::string sha;
    std::string chunk;
    std::string shown;
    std::string upload;

    void keep() { blobs.saveHeld("file-1", "notes/a"); }

    void look() {
        blobs.read("notes/a", 0, 5);
        shown = blobs.url("notes/a");
    }

    void send() { upload = blobs.upload("notes/a", "PUT", "/uploads", ""); }

    [[OnEvent]] void hearSaved(const BlobSaved& saved) {
        bytes = saved.bytes;
        sha = saved.sha256;
    }

    [[OnEvent]] void hearChunk(const BlobChunkRead& read) { chunk = read.data; }
};
"#;

    fn assert_kept(root: &Path, fetches_files: bool) {
        WebApis::write(
            root,
            "application-events.mjs",
            WebApis::EVENTS_RUNTIME.as_bytes(),
        );
        let kept = Self::SERVICE_DRIVER.replace("__GRANT_STUBS__", WebApis::GRANT_STUBS);
        let driver = if fetches_files {
            format!("{}{kept}", WebApis::FILE_FETCH)
        } else {
            kept
        };
        let report = WebApis::run(root, &driver);
        assert_eq!(
            report,
            serde_json::json!({
                "bytes": 5,
                "sha": "sha-file-1",
                "chunk": "aGVsbG8=",
                "shown": "blob:notes/a",
                "upload": "PUT-1",
                "asked": [
                    ["saveHeld", "file-1", "notes/a"],
                    ["read", "notes/a", 0, 5],
                    ["upload", "notes/a", "PUT", "/uploads", ""],
                ],
            })
        );
    }
}

#[tokio::test]
async fn a_rust_web_service_keeps_reads_and_uploads_blobs() {
    if !WebApis::node_available() || !WebApis::rust_web_target() {
        eprintln!("skipping: node or the wasm32 Rust target is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Rust, "rs", BlobsTests::RUST).await;
    BlobsTests::assert_kept(root.path(), false);
}

#[tokio::test]
async fn a_go_web_service_keeps_reads_and_uploads_blobs() {
    let go = Command::new("go")
        .arg("version")
        .output()
        .is_ok_and(|output| output.status.success());
    if !WebApis::node_available() || !go {
        eprintln!("skipping: node or go is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Go, "go", BlobsTests::GO).await;
    BlobsTests::assert_kept(root.path(), true);
}

#[tokio::test]
async fn a_csharp_web_service_keeps_reads_and_uploads_blobs() {
    if !dotnet_wasm_ready() {
        eprintln!("skipping: node or the .NET wasm-tools workload is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = ("cs", BlobsTests::CSHARP);
    let profile = crate::BuildProfile::Development;
    WebApis::publish_with(root.path(), WebLanguage::CSharp, source, profile).await;
    BlobsTests::assert_kept(root.path(), false);
}

#[tokio::test]
async fn an_fsharp_web_service_keeps_reads_and_uploads_blobs() {
    if !dotnet_wasm_ready() {
        eprintln!("skipping: node or the .NET wasm-tools workload is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = ("fs", BlobsTests::FSHARP);
    let profile = crate::BuildProfile::Development;
    WebApis::publish_with(root.path(), WebLanguage::CSharp, source, profile).await;
    BlobsTests::assert_kept(root.path(), false);
}

#[tokio::test]
async fn a_dart_service_keeps_reads_and_uploads_blobs() {
    if !WebApis::node_available() || !WebApis::dart_available() {
        eprintln!("skipping: node or dart is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish_dart(root.path(), BlobsTests::DART).await;
    BlobsTests::assert_kept(root.path(), true);
}

#[tokio::test]
async fn a_ruby_web_service_keeps_reads_and_uploads_blobs() {
    let unavailable = crate::tac_ruby::RubyRuntimeBuild::unavailable_reason();
    if !WebApis::node_available() || unavailable.is_some() {
        eprintln!("skipping: node or the Ruby runtime is unavailable: {unavailable:?}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Ruby, "rb", BlobsTests::RUBY).await;
    BlobsTests::assert_kept(root.path(), true);
}

#[tokio::test]
async fn a_lua_web_service_keeps_reads_and_uploads_blobs() {
    if lua_unavailable() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Lua, "lua", BlobsTests::LUA).await;
    BlobsTests::assert_kept(root.path(), true);
}

#[tokio::test]
async fn a_swift_web_service_keeps_reads_and_uploads_blobs() {
    let root = tempfile::tempdir().expect("project");
    if !publish_swift(root.path(), BlobsTests::SWIFT).await {
        return;
    }
    BlobsTests::assert_kept(root.path(), true);
}

#[tokio::test]
async fn a_kotlin_web_service_keeps_reads_and_uploads_blobs() {
    if !super::super::kotlin_tests::kotlin_web_ready() {
        eprintln!("skipping: kotlinc-js, the Kotlin wasm stdlib or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Kotlin, "kt", BlobsTests::KOTLIN).await;
    BlobsTests::assert_kept(root.path(), true);
}

#[tokio::test]
async fn a_java_web_service_keeps_reads_and_uploads_blobs() {
    let root = tempfile::tempdir().expect("project");
    if !publish_java(root.path(), BlobsTests::JAVA).await {
        return;
    }
    BlobsTests::assert_kept(root.path(), true);
}

#[tokio::test]
async fn a_cpp_web_service_keeps_reads_and_uploads_blobs() {
    if !WebApis::node_available() || !WebApis::cpp_available() {
        eprintln!("skipping: node or em++ is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Cpp, "hpp", BlobsTests::CPP).await;
    BlobsTests::assert_kept(root.path(), true);
}
