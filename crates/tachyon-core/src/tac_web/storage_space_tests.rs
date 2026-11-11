use super::{
    WebApis, WebLanguage, dotnet_wasm_ready, lua_unavailable, publish_java, publish_swift,
};
use std::path::Path;
use std::process::Command;

struct StorageSpaceTests;

impl StorageSpaceTests {
    const RUNTIME_DRIVER: &str = r"__GRANT_STUBS__
;(0, eval)(`${await readFile('./tac-web.js', 'utf8')}\nglobalThis.TacWebApis = TacWebApis`)
const web = globalThis[Symbol.for('tachyon.tac.web')]
TacWebApis.events = { publish: () => {}, send: (owner, name, value) => effects.sent.push([owner, name, value]) }
const ask = (owner, call) => web.calling(owner, () => JSON.parse(web.host(JSON.stringify({ api: 'storageSpace', call, args: [] }))))
const tick = () => new Promise((resolve) => setTimeout(resolve, 0))
const answers = {}
grants.content = ''
answers.insecureEstimate = ask('insecure-estimate', 'estimate')
answers.ungranted = ask('ungranted', 'persist')
await tick()
grants.content = 'persistentStorage'
answers.insecurePersist = ask('insecure-persist', 'persist')
await tick()
TacWebApis.countSpace(async () => 300)
TacWebApis.countSpace(() => 12)
ask('counted-estimate', 'estimate')
await tick()
navigator.storage = { estimate: async () => ({ usage: 2048, quota: 4096 }), persisted: async () => true, persist: async () => true }
ask('web-estimate', 'estimate')
ask('web-persist', 'persist')
await tick()
const calls = []
globalThis.__tachyonNativeHostCall = async (capability, payload) => {
  calls.push([capability, JSON.parse(payload)])
  return JSON.stringify({ value: capability.endsWith('.estimate') ? { quota: 9000, persisted: true } : true })
}
navigator.storage = { estimate: async () => ({ usage: 512, quota: 1024 }), persisted: async () => false, persist: async () => false }
ask('native-estimate', 'estimate')
ask('native-persist', 'persist')
await tick()
delete navigator.storage
ask('apple-estimate', 'estimate')
await tick()
globalThis.__tachyonNativeHostCall = async () => JSON.stringify({ error: 'No disk' })
ask('failed-estimate', 'estimate')
await tick()
for (const list of [effects.sent, calls]) list.sort((left, right) => JSON.stringify(left).localeCompare(JSON.stringify(right)))
console.log(JSON.stringify({ answers, sent: effects.sent, calls }))
";

    const SERVICE_DRIVER: &str = r";(0, eval)(`${await readFile('./tac-web.js', 'utf8')}\nglobalThis.TacWebApis = TacWebApis`)
const { default: TacApplicationEvents } = await import('./application-events.mjs')
const { default: StampService, __tachyon_event_plan__: plan } = await import('./StampService.js')
__GRANT_STUBS__
grants.content = 'persistentStorage'
navigator.storage = { estimate: async () => ({ usage: 2048, quota: 4096 }), persisted: async () => false, persist: async () => true }
const bus = new TacApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })
TacWebApis.events = { publish: (name, value) => bus.publish(name, value), send: (owner, name, value) => bus.send(owner, name, value) }
const stamp = new StampService({})
bus.attach(stamp, plan.types[0], { applicationEventPublisher: null })
await bus.initialize(stamp)
stamp.measure()
stamp.keep()
await new Promise((resolve) => setTimeout(resolve, 0))
console.log(JSON.stringify({ usage: stamp.usage, kept: stamp.kept }))
process.exit(0)
";

    const RUST: &str = r"#[Service]
#[derive(Default)]
struct StampService {
    storage_space: StorageSpace,
    usage: f64,
    kept: bool,
}

impl StampService {
    pub fn measure(&self) {
        let _ = self.storage_space.estimate();
    }

    pub fn keep(&self) {
        let _ = self.storage_space.persist();
    }

    #[OnEvent]
    pub fn hear_estimate(&mut self, estimated: StorageEstimated) {
        self.usage = estimated.usage;
    }

    #[OnEvent]
    pub fn hear_persisted(&mut self, persisted: StoragePersisted) {
        self.kept = persisted.persisted;
    }
}
";

    const GO: &str = r"// @Service
type StampService struct {
	// @Needs
	storageSpace StorageSpace
	Usage        float64
	Kept         bool
}

func (stamp *StampService) Measure() {
	_ = stamp.storageSpace.Estimate()
}

func (stamp *StampService) Keep() {
	_ = stamp.storageSpace.Persist()
}

// @OnEvent
func (stamp *StampService) HearEstimate(estimated StorageEstimated) {
	stamp.Usage = estimated.Usage
}

// @OnEvent
func (stamp *StampService) HearPersisted(persisted StoragePersisted) {
	stamp.Kept = persisted.Persisted
}
";

    const CSHARP: &str = r"[Service]
public class StampService
{
    private readonly StorageSpace storageSpace = new();
    public double Usage = 0;
    public bool Kept = false;

    public void Measure() => storageSpace.Estimate();

    public void Keep() => storageSpace.Persist();

    [OnEvent]
    public void HearEstimate(StorageEstimated estimated)
    {
        Usage = estimated.Usage;
    }

    [OnEvent]
    public void HearPersisted(StoragePersisted persisted)
    {
        Kept = persisted.Persisted;
    }
}
";

    const FSHARP: &str = r"[<Service>]
type StampService() =
    let storageSpace = StorageSpace()
    member val Usage: float = 0.0 with get, set
    member val Kept: bool = false with get, set
    member this.Measure() : unit = storageSpace.Estimate()
    member this.Keep() : unit = storageSpace.Persist()
    [<OnEvent>]
    member this.HearEstimate(estimated: StorageEstimated) : unit = this.Usage <- estimated.Usage
    [<OnEvent>]
    member this.HearPersisted(persisted: StoragePersisted) : unit = this.Kept <- persisted.Persisted
";

    const SWIFT: &str = r"@Service
final class StampService {
    private let storageSpace = StorageSpace()
    var usage: Double = 0
    var kept: Bool = false

    func measure() throws {
        try storageSpace.estimate()
    }

    func keep() throws {
        try storageSpace.persist()
    }

    @OnEvent
    func hearEstimate(_ estimated: StorageEstimated) {
        usage = estimated.usage
    }

    @OnEvent
    func hearPersisted(_ persisted: StoragePersisted) {
        kept = persisted.persisted
    }
}
";

    const KOTLIN: &str = r"@Service
class StampService {
    private val storageSpace = StorageSpace()
    var usage: Double = 0.0
    var kept: Boolean = false

    fun measure() {
        storageSpace.estimate()
    }

    fun keep() {
        storageSpace.persist()
    }

    @OnEvent
    fun hearEstimate(estimated: StorageEstimated) {
        usage = estimated.usage
    }

    @OnEvent
    fun hearPersisted(persisted: StoragePersisted) {
        kept = persisted.persisted
    }
}
";

    const JAVA: &str = r"@Service
class StampService {
    private final StorageSpace storageSpace = new StorageSpace();
    double usage = 0;
    boolean kept = false;

    void measure() {
        storageSpace.estimate();
    }

    void keep() {
        storageSpace.persist();
    }

    @OnEvent
    void hearEstimate(StorageEstimated estimated) {
        usage = estimated.usage();
    }

    @OnEvent
    void hearPersisted(StoragePersisted persisted) {
        kept = persisted.persisted();
    }
}
";

    const DART: &str = r"@Service()
class StampService {
  @Needs()
  final StorageSpace storageSpace;
  double usage = 0;
  bool kept = false;

  void measure() => storageSpace.estimate();

  void keep() => storageSpace.persist();

  @OnEvent
  void hearEstimate(StorageEstimated estimated) {
    usage = estimated.usage;
  }

  @OnEvent
  void hearPersisted(StoragePersisted persisted) {
    kept = persisted.persisted;
  }
}
";

    const RUBY: &str = r"#[Service]
class StampService
  attr_accessor :usage, :kept

  def initialize
    @usage = 0.0
    @kept = false
  end

  def measure
    StorageSpace.new.estimate
  end

  def keep
    StorageSpace.new.persist
  end

  # @param estimated [StorageEstimated]
  #[OnEvent]
  def hear_estimate(estimated)
    @usage = estimated.usage
  end

  # @param persisted [StoragePersisted]
  #[OnEvent]
  def hear_persisted(persisted)
    @kept = persisted.persisted
  end
end
";

    const LUA: &str = r"-- @Service
---@class StampService
StampService = {
  ---@type number
  usage = 0.0,
  ---@type boolean
  kept = false,
}

---@return nil
function StampService:measure()
  StorageSpace:estimate()
end

---@return nil
function StampService:keep()
  StorageSpace:persist()
end

---@param estimated StorageEstimated
---@return nil
-- @OnEvent
function StampService:hear_estimate(estimated)
  self.usage = estimated.usage
end

---@param persisted StoragePersisted
---@return nil
-- @OnEvent
function StampService:hear_persisted(persisted)
  self.kept = persisted.persisted
end
";

    const CPP: &str = r"#pragma once

class [[Service]] StampService {
public:
    [[Needs]] StorageSpace& storageSpace;
    double usage = 0;
    bool kept = false;

    void measure() { storageSpace.estimate(); }

    void keep() { storageSpace.persist(); }

    [[OnEvent]] void hearEstimate(const StorageEstimated& estimated) { usage = estimated.usage; }

    [[OnEvent]] void hearPersisted(const StoragePersisted& persisted) { kept = persisted.persisted; }
};
";

    fn assert_measured(root: &Path, fetches_files: bool) {
        WebApis::write(
            root,
            "application-events.mjs",
            WebApis::EVENTS_RUNTIME.as_bytes(),
        );
        let measured = Self::SERVICE_DRIVER.replace("__GRANT_STUBS__", WebApis::GRANT_STUBS);
        let driver = if fetches_files {
            format!("{}{measured}", WebApis::FILE_FETCH)
        } else {
            measured
        };
        let report = WebApis::run(root, &driver);
        assert_eq!(report, serde_json::json!({"usage": 2048, "kept": true}));
    }
}

#[test]
fn the_page_runtime_estimates_and_persists_storage_on_the_web_and_in_native_apps() {
    if !WebApis::node_available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("root");
    let driver = StorageSpaceTests::RUNTIME_DRIVER.replace("__GRANT_STUBS__", WebApis::GRANT_STUBS);
    let report = WebApis::run(root.path(), &driver);
    let started = serde_json::json!({"value": null});
    assert_eq!(report["answers"]["insecureEstimate"], started, "{report}");
    assert_eq!(report["answers"]["insecurePersist"], started, "{report}");
    assert_eq!(
        report["answers"]["ungranted"]["error"],
        "The persistentStorage browser API is not granted. Add \"persistentStorage\" to tac.browserApis in client/manifest.json."
    );
    assert_eq!(
        report["sent"],
        serde_json::json!([
            ["apple-estimate", "StorageEstimated", {"usage": 312, "quota": 9000, "persisted": true}],
            ["counted-estimate", "StorageEstimated", {"usage": 312, "quota": 0, "persisted": false}],
            ["failed-estimate", "BrowserCallFailed", {"api": "storageSpace", "call": "estimate", "message": "No disk"}],
            ["insecure-estimate", "StorageEstimated", {"usage": 0, "quota": 0, "persisted": false}],
            ["insecure-persist", "StoragePersisted", {"persisted": false}],
            ["native-estimate", "StorageEstimated", {"usage": 512, "quota": 9000, "persisted": true}],
            ["native-persist", "StoragePersisted", {"persisted": true}],
            ["web-estimate", "StorageEstimated", {"usage": 2048, "quota": 4096, "persisted": true}],
            ["web-persist", "StoragePersisted", {"persisted": true}],
        ]),
        "{report}"
    );
    assert_eq!(
        report["calls"],
        serde_json::json!([
            ["browserApi.storageSpace.estimate", {}],
            ["browserApi.storageSpace.estimate", {}],
            ["browserApi.storageSpace.persist", {}],
        ])
    );
}

#[tokio::test]
async fn a_rust_web_service_estimates_and_persists_storage() {
    if !WebApis::node_available() || !WebApis::rust_web_target() {
        eprintln!("skipping: node or the wasm32 Rust target is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(
        root.path(),
        WebLanguage::Rust,
        "rs",
        StorageSpaceTests::RUST,
    )
    .await;
    StorageSpaceTests::assert_measured(root.path(), false);
}

#[tokio::test]
async fn a_go_web_service_estimates_and_persists_storage() {
    let go = Command::new("go")
        .arg("version")
        .output()
        .is_ok_and(|output| output.status.success());
    if !WebApis::node_available() || !go {
        eprintln!("skipping: node or go is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Go, "go", StorageSpaceTests::GO).await;
    StorageSpaceTests::assert_measured(root.path(), true);
}

#[tokio::test]
async fn a_csharp_web_service_estimates_and_persists_storage() {
    if !dotnet_wasm_ready() {
        eprintln!("skipping: node or the .NET wasm-tools workload is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = ("cs", StorageSpaceTests::CSHARP);
    let profile = crate::BuildProfile::Development;
    WebApis::publish_with(root.path(), WebLanguage::CSharp, source, profile).await;
    StorageSpaceTests::assert_measured(root.path(), false);
}

#[tokio::test]
async fn an_fsharp_web_service_estimates_and_persists_storage() {
    if !dotnet_wasm_ready() {
        eprintln!("skipping: node or the .NET wasm-tools workload is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = ("fs", StorageSpaceTests::FSHARP);
    let profile = crate::BuildProfile::Development;
    WebApis::publish_with(root.path(), WebLanguage::CSharp, source, profile).await;
    StorageSpaceTests::assert_measured(root.path(), false);
}

#[tokio::test]
async fn a_dart_service_estimates_and_persists_storage() {
    if !WebApis::node_available() || !WebApis::dart_available() {
        eprintln!("skipping: node or dart is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish_dart(root.path(), StorageSpaceTests::DART).await;
    StorageSpaceTests::assert_measured(root.path(), true);
}

#[tokio::test]
async fn a_ruby_web_service_estimates_and_persists_storage() {
    let unavailable = crate::tac_ruby::RubyRuntimeBuild::unavailable_reason();
    if !WebApis::node_available() || unavailable.is_some() {
        eprintln!("skipping: node or the Ruby runtime is unavailable: {unavailable:?}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(
        root.path(),
        WebLanguage::Ruby,
        "rb",
        StorageSpaceTests::RUBY,
    )
    .await;
    StorageSpaceTests::assert_measured(root.path(), true);
}

#[tokio::test]
async fn a_lua_web_service_estimates_and_persists_storage() {
    if lua_unavailable() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Lua, "lua", StorageSpaceTests::LUA).await;
    StorageSpaceTests::assert_measured(root.path(), true);
}

#[tokio::test]
async fn a_swift_web_service_estimates_and_persists_storage() {
    let root = tempfile::tempdir().expect("project");
    if !publish_swift(root.path(), StorageSpaceTests::SWIFT).await {
        return;
    }
    StorageSpaceTests::assert_measured(root.path(), true);
}

#[tokio::test]
async fn a_kotlin_web_service_estimates_and_persists_storage() {
    if !super::super::kotlin_tests::kotlin_web_ready() {
        eprintln!("skipping: kotlinc-js, the Kotlin wasm stdlib or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(
        root.path(),
        WebLanguage::Kotlin,
        "kt",
        StorageSpaceTests::KOTLIN,
    )
    .await;
    StorageSpaceTests::assert_measured(root.path(), true);
}

#[tokio::test]
async fn a_java_web_service_estimates_and_persists_storage() {
    let root = tempfile::tempdir().expect("project");
    if !publish_java(root.path(), StorageSpaceTests::JAVA).await {
        return;
    }
    StorageSpaceTests::assert_measured(root.path(), true);
}

#[tokio::test]
async fn a_cpp_web_service_estimates_and_persists_storage() {
    if !WebApis::node_available() || !WebApis::cpp_available() {
        eprintln!("skipping: node or em++ is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Cpp, "hpp", StorageSpaceTests::CPP).await;
    StorageSpaceTests::assert_measured(root.path(), true);
}
