#!/usr/bin/env node

import { CargoArtifacts } from './cargo-artifacts.mjs'
import { chromium, firefox, webkit } from 'playwright';
import { createServer } from 'node:http';
import { createServer as createSecureServer } from 'node:https';
import { spawnSync } from 'node:child_process';
import { createReadStream, existsSync, mkdtempSync, mkdirSync, readFileSync, statSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

class StorageGate {
  static repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('release/ty');
  static host = 'tachyon.test';
  static hostRule = `--host-resolver-rules=MAP ${StorageGate.host} 127.0.0.1`;
  static browsers = (process.env.TAC_STORAGE_BROWSERS ?? 'chromium,firefox,webkit').split(',');
  static settleCaches = () => globalThis[Symbol.for('tachyon.tac.storage')].settled();
  static withCSharp = csharpWebAvailable();
  static withKotlin = kotlinWebAvailable();
  static withSwift = swiftWebAvailable();
  static withJava = javaWebAvailable();
  static scriptBudgetKilobytes = 320;
  static mimeTypes = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.wasm': 'application/wasm' };

  static open(name, project) {
    if (name === 'chromium') return chromium.launch({ timeout: 30_000, args: [StorageGate.hostRule] });
    if (name === 'firefox') return firefox.launch({ timeout: 30_000 });
    if (name === 'webkit') return StorageGate.persistentWebKit(project);
    throw new Error(`the storage gate has no browser named '${name}'`);
  }

  static persistentWebKit(project) {
    const contexts = [];
    return {
      newContext: async (options) => {
        const profile = mkdtempSync(path.join(project, 'webkit-profile-'));
        const context = await webkit.launchPersistentContext(profile, { timeout: 30_000, ...options });
        contexts.push(context);
        return context;
      },
      close: () => Promise.all(contexts.map((context) => context.close())),
    };
  }
}

class ApiRequests {
  static counter = 0;
  static rangeRequests = 0;
}

function write(project, relative, contents) {
  const file = path.join(project, relative);
  mkdirSync(path.dirname(file), { recursive: true });
  writeFileSync(file, contents);
}

const listen = (server) => new Promise((resolve, reject) => {
  server.once('error', reject);
  server.listen(0, '127.0.0.1', () => { server.off('error', reject); resolve(); });
});
const closeServer = async (server) => {
  if (!server) return;
  const closed = new Promise((resolve, reject) => server.close((error) => {
    if (error && error.code !== 'ERR_SERVER_NOT_RUNNING') reject(error);
    else resolve();
  }));
  server.closeAllConnections();
  await closed;
};

function writeGoFixture(project) {
  write(project, 'client/setups/GoNotesSetup.go', `// @Setup
type GoNotesSetup struct{}

func (GoNotesSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
\treturn NewBehaviorConfiguration(), nil
}
`);
  write(project, 'client/entities/GoNoteEntity.go', `// @Entity({"adapter":"indexeddb"})
// @Table("goNotes")
type GoNoteEntity struct {
\t// @Id
\tid   string
\ttext string
}
`);
  write(project, 'client/components/storage/go/tac.html', '<p id="go">{goNotes.reads}</p>\n<button id="go-bump" on:click="goNotes.increment()">+</button>\n<p id="go-count">{goNotes.count}</p>\n');
  write(project, 'client/stores/GoNoteStore.go', `// @Store(GoNoteEntity, {"fields":["id","text"],"defaultSize":20,"maxSize":100,"setup":"GoNotesSetup"})
type GoNoteStore interface {
\tFindByText(text BehaviorValue) (BehaviorValue, error)
}
`);
  write(project, 'client/services/GoNotesService.go', `// @CacheSettings({"name":"gonotes","ttlMs":60000,"maxBytes":4096,"maxEntries":8})
type GoNotesService struct {
\tbehaviors *BehaviorContext
\t// @Needs
\tnoteStore *GoNoteStore
\tReads     int
\t// @LocalState
\tCount int
}

func (notes *GoNotesService) Kept() (BehaviorValue, error) {
\treturn notes.noteStore.FindByText("kept")
}

func (notes *GoNotesService) Increment() int {
\tnotes.Count++
\treturn notes.Count
}

// @OnStart
func (notes *GoNotesService) Start() error {
\tcontext, err := BehaviorsCreate[GoNotesSetup](NewBehaviorOptions("go-notes", NewBehaviorSignal()))
\tnotes.behaviors = context
\treturn err
}

// @Cached
func (notes *GoNotesService) Find(key BehaviorValue) (BehaviorValue, error) {
\tnotes.Reads++
\treturn map[string]any{"key": key, "reads": notes.Reads}, nil
}

func (notes *GoNotesService) Save(id BehaviorValue, text BehaviorValue) (BehaviorValue, error) {
\treturn notes.useRepository(func(repository *BehaviorRepository) (BehaviorValue, error) {
\t\treturn repository.Save(map[string]any{"id": id, "text": text})
\t})
}

func (notes *GoNotesService) List() (BehaviorValue, error) {
\treturn notes.useRepository(func(repository *BehaviorRepository) (BehaviorValue, error) {
\t\treturn repository.Find(map[string]any{})
\t})
}

func (notes *GoNotesService) useRepository(action func(*BehaviorRepository) (BehaviorValue, error)) (BehaviorValue, error) {
\trepository, err := BehaviorsRepository[GoNoteEntity](notes.behaviors)
\tif err != nil {
\t\treturn nil, err
\t}
\treturn action(repository)
}
`);
}

function writeRubyFixture(project) {
  write(project, 'client/setups/RubyNotesSetup.rb', '#[Setup]\nclass RubyNotesSetup\n  def self.configure(_execution)\n    {}\n  end\nend\n');
  write(project, 'client/entities/RubyNoteEntity.rb', '#[Entity({"adapter":"indexeddb"})]\n#[Table("rubyNotes")]\nclass RubyNoteEntity\n  # @return [String]\n  #[Id]\n  attr_reader :id\n  # @return [String]\n  attr_reader :text\nend\n');
  write(project, 'client/components/storage/ruby/tac.html', '<p id="ruby">{rubyNotes.reads}</p>\n<button id="ruby-bump" on:click="rubyNotes.increment()">+</button>\n<p id="ruby-count">{rubyNotes.count}</p>\n');
  write(project, 'client/stores/RubyNoteStore.rb', `#[Store(RubyNoteEntity, {"fields":["id","text"],"defaultSize":20,"maxSize":100,"setup":"RubyNotesSetup"})]
class RubyNoteStore
  def find_by_text(text)
  end
end
`);
  write(project, 'client/services/RubyNotesService.rb', `#[Service]
#[CacheSettings({"name":"rubynotes","ttlMs":60000,"maxBytes":4096,"maxEntries":8})]
class RubyNotesService
  attr_reader :reads
  #[Needs]
  # @return [RubyNoteStore]
  attr_reader :ruby_note_store

  def kept
    ruby_note_store.find_by_text("kept")
  end
  #[LocalState]
  attr_accessor :count

  #[OnStart]
  def start
    @reads = 0
    @count = 0
    @behaviors = Behaviors.create(RubyNotesSetup, partition: "ruby-notes", signal: BehaviorSignal.new)
  end

  #[Cached]
  def find(key)
    @reads += 1
    { "key" => key, "reads" => @reads }
  end

  #[ClearCache({"method":"find"})]
  def forget(key)
    nil
  end

  def save(id, text)
    @behaviors.repository(RubyNoteEntity).save({ "id" => id, "text" => text })
  end

  #[AllOrNothing({"adapter":"indexeddb"})]
  def save_then_refuse(id)
    save(id, "draft")
    raise BehaviorError.new("validation", "the note was refused")
  end

  def list
    @behaviors.repository(RubyNoteEntity).find({})
  end

  def increment
    @count += 1
  end
end
`);
}

function writeRustFixture(project) {
  write(project, 'client/components/storage/rs/tac.html', '<button id="rs-bump" on:click="rsCounter.increment()">+</button>\n<p id="rs-count">{rsCounter.count}</p>\n');
  write(project, 'client/setups/RsNotesSetup.rs', `#[Setup]
struct RsNotesSetup {}
impl RsNotesSetup {
    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {
        Ok(BehaviorConfiguration::default())
    }
}
`);
  write(project, 'client/entities/RsNoteEntity.rs', `#[Entity({"adapter":"indexeddb"})]
#[Table("rsNotes")]
struct RsNoteEntity {
    #[Id]
    id: String,
    text: String,
}
`);
  write(project, 'client/stores/RsNoteStore.rs', `#[Store(RsNoteEntity, {"fields":["id","text"],"defaultSize":20,"maxSize":100,"setup":"RsNotesSetup"})]
trait RsNoteStore {
    fn count_by_text(&self, text: BehaviorValue) -> BehaviorResult<BehaviorValue>;
}
`);
  write(project, 'client/services/RsCounterService.rs', `#[Service]
#[derive(Default)]
struct RsCounterService {
    #[LocalState]
    count: i64,
    #[Needs]
    note_store: std::sync::Arc<RsNoteStore>,
}

impl RsCounterService {
    fn kept_count(&self) -> f64 {
        match self.note_store.count_by_text(BehaviorValue::String(String::from("kept"))) {
            Ok(BehaviorValue::Number(count)) => count,
            _ => -1.0,
        }
    }

    fn increment(&mut self) -> i64 {
        self.count += 1;
        self.count
    }
}
`);
}

function swiftWebAvailable() {
  const sdk = process.env.TAC_SWIFT_WASM_SDK;
  return Boolean(sdk) && existsSync(path.join(sdk, 'WASI.sdk'));
}

function writeSwiftFixture(project) {
  write(project, 'client/setups/SwiftNotesSetup.swift', `@Setup
final class SwiftNotesSetup {
    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters {
        BehaviorAdapters()
    }
}
`);
  write(project, 'client/entities/SwiftNoteEntity.swift', `@Entity({"adapter":"indexeddb"})
@Table("swiftNotes")
final class SwiftNoteEntity {
    @Id
    var id: String = ""
    var text: String = ""
}
`);
  write(project, 'client/components/storage/swift/tac.html', '<button id="swift-bump" on:click="swiftCounter.increment()">+</button>\n<p id="swift-count">{swiftCounter.count}</p>\n');
  write(project, 'client/stores/SwiftNoteStore.swift', `@Store(SwiftNoteEntity, {"fields":["id","text"],"defaultSize":20,"maxSize":100,"setup":"SwiftNotesSetup"})
protocol SwiftNoteStore {
    func findByText(_ text: BehaviorJSON) async throws -> BehaviorJSON
}
`);
  write(project, 'client/services/SwiftCounterService.swift', `@Service
@CacheSettings({"name":"swiftnotes","ttlMs":60000,"maxBytes":4096,"maxEntries":8})
final class SwiftCounterService {
    @LocalState
    var count: Int = 0
    private var behaviors: BehaviorContext!
    private var reads: Int = 0
    @Needs
    let noteStore: SwiftNoteStore

    func kept() async throws -> BehaviorJSON {
        try await noteStore.findByText(.string("kept"))
    }

    @OnStart
    func open() {
        behaviors = try? Behaviors.create(SwiftNotesSetup.self, partition: "swift-notes", signal: BehaviorSignal())
    }

    func increment() -> Int {
        count += 1
        return count
    }

    @Cached
    func find(_ key: BehaviorJSON) async throws -> BehaviorJSON {
        reads += 1
        return .object(["key": key, "reads": .number(Double(reads))])
    }

    @ClearCache({"method":"find"})
    func forget(_ key: BehaviorJSON) async throws -> BehaviorJSON {
        key
    }

    func save(_ id: String, _ text: String) async throws -> BehaviorJSON {
        try await notes().save(.object(["id": .string(id), "text": .string(text)]))
    }

    @AllOrNothing({"adapter":"indexeddb"})
    func saveThenRefuse(_ id: String) async throws -> BehaviorJSON {
        _ = try await save(id, "draft")
        throw BehaviorError("the note was refused")
    }

    func list() async throws -> BehaviorJSON {
        try await notes().find()
    }

    private func notes() throws -> BehaviorRepository {
        try behaviors.repository(SwiftNoteEntity.self)
    }
}
`);
}

function javaWebAvailable() {
  const home = process.env.TAC_TEAVM_HOME;
  return Boolean(home) && existsSync(home);
}

function writeJavaFixture(project) {
  write(project, 'client/setups/JavaNotesSetup.java', `@Setup
class JavaNotesSetup {
    static Behaviors.Configuration configure(Behaviors.Execution execution) {
        return new Behaviors.Configuration();
    }
}
`);
  write(project, 'client/entities/JavaNoteEntity.java', `@Entity({"adapter":"indexeddb"})
@Table("javaNotes")
class JavaNoteEntity {
    @Id
    String id = "";
    String text = "";
}
`);
  write(project, 'client/components/storage/java/tac.html', '<button id="java-bump" on:click="javaCounter.increment()">+</button>\n<p id="java-count">{javaCounter.count}</p>\n');
  write(project, 'client/stores/JavaNoteStore.java', `@Store(JavaNoteEntity.class, {"fields":["id","text"],"defaultSize":20,"maxSize":100,"setup":"JavaNotesSetup"})
interface JavaNoteStore {
    Object findByText(String text);
}
`);
  write(project, 'client/services/JavaCounterService.java', `import java.util.Map;

@Service
@CacheSettings({"name":"javanotes","ttlMs":60000,"maxBytes":4096,"maxEntries":8})
class JavaCounterService {
    @LocalState
    int count = 0;
    Behaviors.Context behaviors;
    private long reads = 0;
    @Needs
    JavaNoteStore noteStore;

    Object kept() {
        return noteStore.findByText("kept");
    }

    @OnStart
    void open() {
        behaviors = Behaviors.create("JavaNotesSetup", new Behaviors.Options("java-notes", new Behaviors.Signal()));
    }

    int increment() {
        count += 1;
        return count;
    }

    @Cached
    Object find(String key) {
        reads += 1;
        return Map.of("key", key, "reads", reads);
    }

    @ClearCache({"method":"find"})
    Object forget(String key) {
        return key;
    }

    Object save(String id, String text) {
        return notes().save(Map.of("id", id, "text", text));
    }

    @AllOrNothing({"adapter":"indexeddb"})
    Object saveThenRefuse(String id) {
        save(id, "draft");
        throw new IllegalStateException("the note was refused");
    }

    Object list() {
        return notes().find(Map.<String, Object>of("size", 10));
    }

    private Behaviors.Repository notes() {
        return behaviors.repository("JavaNoteEntity");
    }
}
`);
}

function csharpWebAvailable() {
  const probe = spawnSync('dotnet', ['workload', 'list'], { encoding: 'utf8', timeout: 30_000, maxBuffer: 1024 * 1024 });
  return !probe.error && probe.status === 0 && probe.stdout.includes('wasm-tools');
}

function kotlinWebAvailable() {
  const library = process.env.TAC_KOTLIN_WASM_STDLIB;
  if (!library || !existsSync(library)) return false;
  const probe = spawnSync('kotlinc-js', ['-version'], { encoding: 'utf8', timeout: 60_000, maxBuffer: 1024 * 1024 });
  return !probe.error && probe.status === 0;
}

function writeKotlinFixture(project) {
  write(project, 'client/setups/KtNotesSetup.kt', `@Setup
class KtNotesSetup {
    companion object {
        fun configure(execution: Behaviors.Execution): Behaviors.Configuration = Behaviors.Configuration()
    }
}
`);
  write(project, 'client/entities/KtNoteEntity.kt', `@Entity({"adapter":"indexeddb"})
@Table("ktNotes")
class KtNoteEntity {
    @Id
    var id: String = ""
    var text: String = ""
}
`);
  write(project, 'client/components/storage/kt/tac.html', '<p id="kt">{ktNotes.reads}</p>\n<button id="kt-bump" on:click="ktNotes.increment()">+</button>\n<p id="kt-count">{ktNotes.count}</p>\n');
  write(project, 'client/stores/KtNoteStore.kt', `@Store(KtNoteEntity::class, {"fields":["id","text"],"defaultSize":20,"maxSize":100,"setup":"KtNotesSetup"})
interface KtNoteStore {
    fun findByText(text: String): Any?
}
`);
  write(project, 'client/services/KtNotesService.kt', `@Service
@CacheSettings({"name":"ktnotes","ttlMs":60000,"maxBytes":4096,"maxEntries":8})
class KtNotesService {
    lateinit var behaviors: Behaviors.Context
    var reads: Long = 0
    @Needs
    val noteStore: KtNoteStore

    fun kept(): Any? = noteStore.findByText("kept")
    @LocalState
    var count: Long = 0

    fun increment(): Long {
        count += 1
        return count
    }

    @OnStart
    fun open() {
        behaviors = Behaviors.create("KtNotesSetup", Behaviors.Options("kt-notes", Behaviors.Signal()))
    }

    @Cached
    fun find(key: String): Any? {
        reads += 1
        return mapOf("key" to key, "reads" to reads)
    }

    @ClearCache({"method":"find"})
    fun forget(key: String): Any? {
        return key
    }

    fun save(id: String, text: String): Any? {
        return behaviors.repository(NOTES).save(mapOf("id" to id, "text" to text))
    }

    @AllOrNothing({"adapter":"indexeddb"})
    fun saveThenRefuse(id: String): Any? {
        save(id, "draft")
        throw IllegalStateException("the note was refused")
    }

    fun list(): Any? {
        return behaviors.repository(NOTES).find(emptyMap())
    }

    companion object {
        private const val NOTES = "KtNoteEntity"
    }
}
`);
}

function writeCSharpFixture(project) {
  write(project, 'client/setups/CsNotesSetup.cs', `[Setup]
public sealed class CsNotesSetup {
    public static BehaviorAdapters configure(BehaviorExecution execution) { return new BehaviorAdapters(); }
}
`);
  write(project, 'client/entities/CsNoteEntity.cs', `[Entity({"adapter":"indexeddb"})]
[Table("csNotes")]
public sealed class CsNoteEntity {
    [Id]
    public string id;
    public string text;
}
`);
  write(project, 'client/components/storage/cs/tac.html', '<p id="cs">{csNotes.reads}</p>\n<button id="cs-bump" on:click="csNotes.increment()">+</button>\n<p id="cs-count">{csNotes.count}</p>\n');
  write(project, 'client/stores/CsNoteStore.cs', `[Store(typeof(CsNoteEntity), {"fields":["id","text"],"defaultSize":20,"maxSize":100,"setup":"CsNotesSetup"})]
public interface CsNoteStore {
    System.Threading.Tasks.Task<BehaviorJSON> findByText(BehaviorJSON text);
}
`);
  write(project, 'client/services/CsNotesService.cs', `[Service]
[CacheSettings({"name":"csnotes","ttlMs":60000,"maxBytes":4096,"maxEntries":8})]
public sealed class CsNotesService {
    public BehaviorContext behaviors;
    public int reads = 0;
    [Needs]
    CsNoteStore _noteStore;

    public System.Threading.Tasks.Task<BehaviorJSON> kept() {
        return _noteStore.findByText(BehaviorJSON.From("kept"));
    }
    [LocalState]
    public int count = 0;

    public int increment() {
        count++;
        return count;
    }

    [OnStart]
    public void open() {
        behaviors = Behaviors.create(typeof(CsNotesSetup), "cs-notes", new BehaviorSignal());
    }

    [Cached]
    public async System.Threading.Tasks.Task<BehaviorJSON> find(BehaviorJSON key) {
        await System.Threading.Tasks.Task.CompletedTask;
        reads++;
        return BehaviorJSON.Object(new System.Collections.Generic.Dictionary<string, BehaviorJSON> { ["key"] = key, ["reads"] = BehaviorJSON.From(reads) });
    }

    [ClearCache({"method":"find"})]
    public async System.Threading.Tasks.Task<BehaviorJSON> forget(BehaviorJSON key) {
        await System.Threading.Tasks.Task.CompletedTask;
        return key;
    }

    public System.Threading.Tasks.Task<BehaviorJSON> save(BehaviorJSON id, BehaviorJSON text) {
        return behaviors.repository(typeof(CsNoteEntity)).save(BehaviorJSON.Object(new System.Collections.Generic.Dictionary<string, BehaviorJSON> { ["id"] = id, ["text"] = text }));
    }

    [AllOrNothing({"adapter":"indexeddb"})]
    public async System.Threading.Tasks.Task<BehaviorJSON> refuse(BehaviorJSON id) {
        await save(id, BehaviorJSON.From("draft"));
        throw new BehaviorError("the note was refused");
    }

    public System.Threading.Tasks.Task<BehaviorJSON> list() {
        return behaviors.repository(typeof(CsNoteEntity)).find(BehaviorJSON.Parse("{}"));
    }
}
`);
}

function writeFsharpFixture(project) {
  write(project, 'client/setups/JournalSetup.fs', `[<Setup>]
type JournalSetup() =
    static member configure(execution: BehaviorExecution) : BehaviorAdapters = BehaviorAdapters()
`);
  write(project, 'client/entities/JournalEntity.fs', `[<Entity({"adapter":"indexeddb"})>]
[<Table("journal")>]
type JournalEntity() =
    [<Id>]
    member val id : string = "" with get, set
    member val text : string = "" with get, set
`);
  write(project, 'client/stores/JournalStore.fs', `open System.Threading.Tasks

[<Store(typeof<JournalEntity>, {"fields":["id","text"],"defaultSize":20,"maxSize":100,"setup":"JournalSetup"})>]
type JournalStore =
    inherit Repository<JournalEntity, string>

    abstract findByText: text: BehaviorJSON -> Task<BehaviorJSON>
`);
  write(project, 'client/services/JournalService.fs', `open System.Threading.Tasks

[<Service>]
type JournalService(entries: JournalStore) =
    member val label : string = "F# journal" with get, set

    [<LocalState>]
    member val count : int = 0 with get, set

    member this.increment() : int =
        this.count <- this.count + 1
        this.count

    member this.keep(id: BehaviorJSON, text: BehaviorJSON) : Task<BehaviorJSON> =
        entries.save(BehaviorJSON.Object(dict [ "id", id; "text", text ]))

    member this.kept() : Task<BehaviorJSON> = entries.findByText(BehaviorJSON.From("kept"))
`);
  write(project, 'client/components/storage/journal/tac.html', '<p id="journal">{journal.label}</p>\n<button id="journal-bump" on:click="journal.increment()">+</button>\n<p id="journal-count">{journal.count}</p>\n');
}

function writeFixture(project) {
  write(project, 'client/pages/tac.html', '<storage-page />\n');
  write(project, 'client/manifest.json', JSON.stringify({ tac: { budgets: { script: StorageGate.scriptBudgetKilobytes }, visibility: { check: false } } }));
  write(project, 'client/components/storage/page/tac.html', `<main aria-label="Storage">
  <button id="bump" on:click="storagePage.clicks += 1">bump</button>
  <button id="dark" on:click="storagePage.theme = 'dark'">dark</button>
  <p id="clicks">{storagePage.clicks}</p>
  <p id="theme">{storagePage.theme}</p>
  <logic :if="storagePage.clicks > 2"><panel-detail /></logic>
  <storage-clients />
  <storage-catalog />
  <storage-notes />
  <storage-store />
  <storage-go />
  <storage-ruby />
  <storage-rs />
${StorageGate.withCSharp ? '  <storage-cs />\n  <storage-journal />\n' : ''}${StorageGate.withKotlin ? '  <storage-kt />\n' : ''}${StorageGate.withSwift ? '  <storage-swift />\n' : ''}${StorageGate.withJava ? '  <storage-java />\n' : ''}</main>
`);
  for (const [name, policy] of [['CacheFirstClient','cache-first'],['NetworkFirstClient','network-first'],['FreshClient','no-store'],['ReloadClient','reload']]) {
    write(project, 'client/clients/' + name + '.js', `@HTTPClient({"cache":"${policy}","adapter":"${name}"})
class ${name} {
  @GetRequest("/api/counter")
  async counter() {}
  @GetRequest("/api/counter")
  async counterAsking(@RequestParam("ambient") ambient) {}
  @GetRequest("/api/counter")
  async counterSigned(@RequestParam("auth") auth, @RequestHeader("authorization") authorization) {}
  @GetRequest("/api/counter")
  async counterWithPolicy(@RequestParam("policy") policy) {}
  @GetRequest("/api/range")
  async range(@RequestParam("sample") sample) {}
  @GetRequest("/api/range")
  async rangePart(@RequestParam("sample") sample, @RequestHeader("Range") range) {}
  @GetRequest("/api/range")
  async rangeIf(@RequestParam("sample") sample, @RequestHeader("If-Range") ifRange) {}
  @GetRequest("/api/range")
  async rangeUnasked(@RequestParam("partial") partial) {}
  @GetRequest("/api/range")
  async rangeLegacy(@RequestParam("legacy") legacy) {}
  @GetRequest("/api/range")
  async rangeMislabelled(@RequestParam("partial") partial, @RequestParam("mislabelled") mislabelled) {}
}`);
  }
  write(project, 'client/services/StoragePageService.js', `@Service()
export default class StoragePageService {
  @SessionState
  clicks = 0;
  @LocalState
  theme = 'light';
}
`);
  write(project, 'client/components/storage/clients/tac.html', '<span hidden>{storageClients.label}</span>\n');
  for (const model of ['CacheWarmed', 'CacheAudited']) {
    write(project, `client/models/${model}.js`, `/**\n * @typedef {Object} ${model}\n * @property {integer} size\n */\nexport {}\n`);
  }
  write(project, 'client/services/StorageClientsService.js', `@Service
export default class StorageClientsService {
  @Needs
  cacheManager;
  @Needs
  applicationEventPublisher;
  @Needs
  cacheFirstClient;
  @Needs
  networkFirstClient;
  @Needs
  freshClient;
  @Needs
  reloadClient;
  label = 'clients';
  announce(size) {
    this.label = 'clients';
    /** @type {CacheWarmed} */
    const warmed = { size };
    this.applicationEventPublisher.publishEvent(warmed);
  }
  /**
   * @param {CacheWarmed} value
   * @returns {CacheAudited}
   */
  @OnEvent({"condition":"value.size >= 2"})
  @Order(1)
  warmed(value) { this.label += '|warmed:' + value.size; return { size: 0 }; }
  /** @param {CacheAudited} value */
  @OnEvent({"condition":"value.size >= 2"})
  checked(value) { this.label += '|audited:' + value.size; }
  /** @param {CacheWarmed} value */
  @OnEvent
  @Order(-1)
  first(value) { this.label += '|first:' + value.size; }
  /** @param {CacheWarmed} value */
  @OnEvent
  @InBackground()
  async audited(value) { this.label += '|async:' + value.size; }
}
`);
  writeKeptFixture(project);
  write(project, 'client/components/panel/detail/tac.html', '<p id="detail">{panelDetail.label}</p>\n');
  write(project, 'client/services/PanelDetailService.js', "@Service()\nexport default class PanelDetailService { label = 'detail'; }\n");
}

function writeKeptFixture(project) {
  write(project, 'client/setups/StorageSetup.js', `@Setup
class StorageSetup {
  static configure() { return {}; }
}
`);
  write(project, 'client/components/storage/catalog/tac.html', '<p id="catalog">{storageCatalog.reads}</p>\n');
  write(project, 'client/services/StorageCatalogService.js', `@Service
@CacheSettings({"name":"catalog","ttlMs":4000,"maxBytes":4096,"maxEntries":2})
export default class StorageCatalogService {
  @Needs
  lifecycle;
  behaviors = null;
  reads = 0;

  @OnStart
  async open() {
    this.behaviors = await Behaviors.create(StorageSetup, { partition: 'catalog', signal: this.lifecycle });
  }

  @Cached
  async find(sku) {
    this.reads += 1;
    return { sku, reads: this.reads };
  }

  @ClearCache({"method":"find"})
  async invalidate(sku) { return { sku }; }
}
`);
  write(project, 'client/entities/NoteEntity.js', `@Entity({"adapter":"indexeddb"})
@Table('notes')
class NoteEntity {
  @Id
  id;
  status;
  rank;
}
`);
  write(project, 'client/stores/NoteStore.js', `@Store(NoteEntity, {"fields":["id","status","rank"],"defaultSize":20,"maxSize":100,"setup":"StorageSetup"})
class NoteStore extends Repository {
  async findByStatus(status) {}
  async countByStatus(status) {}
  async deleteByStatus(status) {}
  @Query({"operation":"find","where":[{"field":"rank","operator":"gte","argument":0}],"orderBy":[{"field":"rank","direction":"desc"}]})
  async ranked(rank, @PageDefaults page) {}
}
`);
  writeGoFixture(project);
  writeRubyFixture(project);
  writeRustFixture(project);
  if (StorageGate.withCSharp) writeCSharpFixture(project);
  if (StorageGate.withCSharp) writeFsharpFixture(project);
  if (StorageGate.withKotlin) writeKotlinFixture(project);
  if (StorageGate.withSwift) writeSwiftFixture(project);
  if (StorageGate.withJava) writeJavaFixture(project);
  write(project, 'client/components/storage/notes/tac.html', '<p id="notes">{storageNotes.label}</p>\n');
  write(project, 'client/components/storage/store/tac.html', '<p id="store-notes">{storeNotes.label}</p>\n');
  write(project, 'client/services/StoreNotesService.js', `@Service
export default class StoreNotesService {
  static OPEN = 'open';
  static SAVED_ID = 'r';
  static GONE = 'gone';
  static LOW_RANK = 2;
  static PAGE_SIZE = 2;
  /** @type {NoteStore} */
  @Needs
  noteStore;
  label = 'store';

  async report() {
    return {
      open: this._ids(await this.noteStore.findByStatus(StoreNotesService.OPEN)),
      count: await this.noteStore.countByStatus(StoreNotesService.OPEN),
      ranked: this._ids(await this.noteStore.ranked(StoreNotesService.LOW_RANK, { size: StoreNotesService.PAGE_SIZE })),
    };
  }

  async repository() {
    const saved = await this.noteStore.save({ id: StoreNotesService.SAVED_ID, status: StoreNotesService.GONE, rank: 0 });
    const found = await this.noteStore.findById(StoreNotesService.SAVED_ID);
    const firstPage = await this.noteStore.findAll({ size: 1 });
    const removed = await this.noteStore.deleteByStatus(StoreNotesService.GONE);
    const missing = await this.noteStore.findById(StoreNotesService.SAVED_ID);
    return { saved: saved.id, found: found.id, firstPage: firstPage.length, removed, missing };
  }

  _ids(rows) {
    return rows.map((row) => row.id);
  }
}
`);
  write(project, 'client/services/StorageNotesService.js', `@Service
export default class StorageNotesService {
  static OPEN = 'open';
  static DELETED_ID = 'b';
  static LOW_RANK = 2;
  static PAGE_SIZE = 2;
  @Needs
  lifecycle;
  behaviors = null;
  label = 'notes';

  @OnStart
  async open() {
    this.behaviors = await Behaviors.create(StorageSetup, { partition: 'notes', signal: this.lifecycle });
  }

  async seed() {
    const notes = this.behaviors.repository(NoteEntity);
    await this.#saveAll(notes, [['a', StorageNotesService.OPEN, 3], [StorageNotesService.DELETED_ID, StorageNotesService.OPEN, 1], ['c', 'done', StorageNotesService.LOW_RANK], ['d', StorageNotesService.OPEN, StorageNotesService.LOW_RANK]]);
  }

  async #saveAll(notes, rows) {
    for (const [id, status, rank] of rows) {
      await notes.save({ id, status, rank });
    }
  }

  @AllOrNothing({"adapter":"indexeddb"})
  async failedBatch() {
    await this.behaviors.repository(NoteEntity).save({ id: 'x', status: StorageNotesService.OPEN, rank: 9 });
    throw new Error('stop');
  }

  @AllOrNothing({"adapter":"indexeddb"})
  async batch() {
    const notes = this.behaviors.repository(NoteEntity);
    await notes.save({ id: 'y', status: StorageNotesService.OPEN, rank: 7 });
    await notes.delete(StorageNotesService.DELETED_ID);
  }

  async report() {
    return {
      page: this.#ids(await this.behaviors.repository(NoteEntity).find({ size: StorageNotesService.PAGE_SIZE, page: 1 })),
    };
  }

  #ids(rows) {
    return rows.map((row) => row.id);
  }
}
`);
}

const rootHelper = () => { globalThis.__root = (name) => __tachyonTac.service(name); };

function buildFixture(project) {
  const built = spawnSync(StorageGate.binary, ['build', project], {
    encoding: 'utf8', timeout: 120_000, killSignal: 'SIGKILL', maxBuffer: 1024 * 1024,
  });
  if (built.status !== 0) {
    throw new Error(`storage fixture build failed: ${built.error?.message || built.stderr || built.stdout}`);
  }
}

function createHandler(project) {
  const root = path.join(project, 'dist');
  return (request, response) => {
    const url = new URL(request.url, 'http://localhost');
    if (url.pathname === '/api/range') {
      ApiRequests.rangeRequests += 1;
      const partial = Boolean(request.headers.range) || url.searchParams.has('partial');
      response.writeHead(partial && !url.searchParams.has('mislabelled') ? 206 : 200, {
        'content-type': 'application/json', 'cache-control': 'public, max-age=60',
        'x-request-number': String(ApiRequests.rangeRequests),
        ...(partial ? { 'content-range': 'bytes 0-2/6' } : {}),
      });
      response.end(JSON.stringify({status:partial && !url.searchParams.has('mislabelled') ? 206 : 200, body:partial ? 'abc' : 'abcdef', number:ApiRequests.rangeRequests}));
      return;
    }
    if (url.pathname === '/api/counter') {
      ApiRequests.counter += 1;
      const cacheControl = url.searchParams.get('policy') || 'public, max-age=60';
      response.writeHead(200, { 'content-type': 'application/json', 'cache-control': cacheControl });
      response.end(JSON.stringify({ value: ApiRequests.counter, cookie: Boolean(request.headers.cookie) }));
      return;
    }
    let file = path.join(root, decodeURIComponent(url.pathname));
    if (existsSync(file) && statSync(file).isDirectory()) file = path.join(file, 'index.html');
    if (!existsSync(file)) { response.writeHead(404).end(); return; }
    response.writeHead(200, { 'content-type': StorageGate.mimeTypes[path.extname(file)] ?? 'application/octet-stream' });
    createReadStream(file).pipe(response);
  };
}

function fail(message) {
  console.error(`FAIL: ${message}`);
  process.exitCode = 1;
}

function expect(actual, wanted, label) {
  if (actual === wanted) console.log(`    ok   ${label}: ${actual}`);
  else fail(`${label}: expected ${JSON.stringify(wanted)}, got ${JSON.stringify(actual)}`);
}

async function checkFieldStorage(context, origin) {
  const page = await context.newPage();
  await page.goto(`${origin}/`, { waitUntil: 'networkidle' });

  expect(await page.textContent('#clicks'), '0', 'declared default renders');
  await page.click('#bump');
  await page.click('#bump');
  await page.click('#bump');
  await page.click('#dark');
  expect(await page.textContent('#clicks'), '3', 'assignment renders');

  await page.reload({ waitUntil: 'networkidle' });
  expect(await page.textContent('#clicks'), '3', 'SessionState field restored after reload');
  expect(await page.textContent('#theme'), 'dark', 'LocalState field restored after reload');
  expect(await page.textContent('#detail'), 'detail', 'restored state re-opened the branch');

  const stored = await page.evaluate(() => ({
    session: sessionStorage.getItem('tac:service:storagePage:/:clicks'),
    local: localStorage.getItem('tac:service:storagePage:/:theme'),
    sessionHasLocalField: sessionStorage.getItem('tac:service:storagePage:/:theme'),
  }));
  expect(stored.session, '3', 'SessionState field written to sessionStorage');
  expect(stored.local, '"dark"', 'LocalState field written to localStorage');
  expect(stored.sessionHasLocalField, null, 'LocalState field kept out of sessionStorage');

  const second = await context.newPage();
  await second.goto(`${origin}/`, { waitUntil: 'networkidle' });
  expect(await second.textContent('#clicks'), '0', 'SessionState field is per-tab');
  expect(await second.textContent('#theme'), 'dark', 'LocalState field is shared across tabs');
  await second.evaluate(() => {
    localStorage.removeItem('tac:service:storagePage:/:theme');
    localStorage.setItem('tac:service:storagePage:/:$$theme', '"sepia"');
  });
  await second.reload({ waitUntil: 'networkidle' });
  expect(await second.textContent('#theme'), 'sepia', 'a value kept under the old $$ key is adopted');
  const adopted = await second.evaluate(() => [localStorage.getItem('tac:service:storagePage:/:$$theme'), localStorage.getItem('tac:service:storagePage:/:theme')]);
  expect(JSON.stringify(adopted), JSON.stringify([null, '"sepia"']), 'the old key moves to the new key');
  await second.evaluate(() => localStorage.setItem('tac:service:storagePage:/:theme', '"dark"'));
  await second.close();
  return page;
}

async function checkKeptCaches(page) {
  const catalog = () => page.evaluate(() => __root('storageCatalog'));
  const find = (sku) => page.evaluate((value) => __root('storageCatalog').find(value), sku);
  await catalog();
  expect((await find('anvil')).reads, 1, 'Cached runs the method on a miss');
  expect((await find('anvil')).reads, 1, 'Cached answers a hit from the built-in cache');
  await page.evaluate(StorageGate.settleCaches);
  const stored = await page.evaluate(async () => {
    const cache = await caches.open('tachyon-cache:catalog');
    return (await cache.keys()).length;
  });
  expect(stored, 1, 'the entry is written to the browser Cache API');
  await page.reload({ waitUntil: 'networkidle' });
  expect((await find('anvil')).reads, 1, 'the entry survives a reload');
  expect(await page.evaluate(() => __root('storageCatalog').reads), 0, 'the reloaded service did not run the method');
  await find('bolt');
  await find('cog');
  await page.evaluate(StorageGate.settleCaches);
  const kept = await page.evaluate(async () => {
    const cache = await caches.open('tachyon-cache:catalog');
    return (await cache.keys()).map((request) => decodeURIComponent(new URL(request.url).pathname)).some((path) => path.includes('anvil'));
  });
  expect(kept, false, 'maxEntries evicts the least recently used entry');
  expect((await find('anvil')).reads > 1, true, 'an evicted entry runs the method again');
  await page.evaluate(() => __root('storageCatalog').invalidate('cog'));
  const cleared = await find('cog');
  expect(cleared.reads, await page.evaluate(() => __root('storageCatalog').reads), 'ClearCache removes the entry');
  const before = await page.evaluate(() => __root('storageCatalog').reads);
  await page.waitForTimeout(4100);
  expect((await find('cog')).reads, before + 1, 'an expired entry runs the method again');
}

async function checkInsecureCaches(browser, port) {
  const insecure = await browser.newContext();
  await insecure.addInitScript(rootHelper);
  const page = await insecure.newPage();
  const notices = [];
  page.on('console', (message) => { if (message.text().includes('memory only')) notices.push(message.text()); });
  await page.goto(`http://${StorageGate.host}:${port}/`, { waitUntil: 'networkidle' });
  expect(await page.evaluate(() => globalThis.isSecureContext), false, 'the plain-http host is an insecure origin');
  const first = await page.evaluate(() => __root('storageCatalog').find('anvil'));
  const second = await page.evaluate(() => __root('storageCatalog').find('anvil'));
  expect(JSON.stringify([first.reads, second.reads]), '[1,1]', 'an insecure origin still caches in memory');
  expect(notices.length, 1, 'one console notice explains the memory-only cache');
  await page.reload({ waitUntil: 'networkidle' });
  expect((await page.evaluate(() => __root('storageCatalog').find('anvil'))).reads, 1, 'a memory-only cache starts empty after a reload');
  await insecure.close();
}

async function checkIndexedStore(browser, origin) {
  const context = await browser.newContext();
  await context.addInitScript(rootHelper);
  const page = await context.newPage();
  await page.goto(`${origin}/.tachyon/tac-client.js`);
  await page.evaluate(() => new Promise((resolve, reject) => {
    const request = indexedDB.open('tachyon-store', 1);
    request.onupgradeneeded = () => { request.result.createObjectStore('notes', { keyPath: 'id' }).put({ id: 'z', status: 'open', rank: 0 }); };
    request.onsuccess = () => { request.result.close(); resolve(); };
    request.onerror = () => reject(request.error);
  }));
  await page.goto(`${origin}/`, { waitUntil: 'networkidle' });
  const notes = (script) => page.evaluate(script);
  await notes(() => __root('storageNotes').seed());
  const upgraded = await notes(async () => {
    const database = await new Promise((resolve) => { const request = indexedDB.open('tachyon-store'); request.onsuccess = () => resolve(request.result); });
    const store = database.transaction('notes').objectStore('notes');
    const answer = { version: database.version, indexes: [...store.indexNames].sort() };
    database.close();
    return answer;
  });
  expect(JSON.stringify(upgraded), JSON.stringify({ version: 2, indexes: ['rank', 'status'] }), 'the schema upgrade adds the query indexes');
  const reports = async () => ({ ...(await __root('storeNotes').report()), ...(await __root('storageNotes').report()) });
  expect(JSON.stringify(await notes(reports)), JSON.stringify({ open: ['a', 'b', 'd', 'z'], count: 4, ranked: ['a', 'c'], page: ['c', 'd'] }), 'derived, Query and paged reads of the injected NoteStore use the stored rows and the row kept before the upgrade');
  const failed = await notes(() => __root('storageNotes').failedBatch().then(() => 'saved', (error) => error.message));
  expect(failed, 'stop', 'AllOrNothing rethrows the failure');
  await notes(() => __root('storageNotes').batch());
  await page.reload({ waitUntil: 'networkidle' });
  expect(JSON.stringify(await notes(reports)), JSON.stringify({ open: ['a', 'd', 'y', 'z'], count: 4, ranked: ['y', 'a'], page: ['d', 'y'] }), 'rows survive a reload; a failed AllOrNothing wrote nothing and a finished one wrote everything');
  expect(JSON.stringify(await notes(() => __root('storeNotes').repository())), JSON.stringify({ saved: 'r', found: 'r', firstPage: 1, removed: 1, missing: null }), 'a NoteStore that extends Repository saves, finds by id, pages and deletes by a derived query in IndexedDB');
  await context.close();

  const changed = await browser.newContext();
  await changed.addInitScript(rootHelper);
  const other = await changed.newPage();
  await other.goto(`${origin}/.tachyon/tac-client.js`);
  await other.evaluate(() => new Promise((resolve) => {
    const request = indexedDB.open('tachyon-store', 1);
    request.onupgradeneeded = () => { request.result.createObjectStore('notes', { keyPath: 'code' }); };
    request.onsuccess = () => { request.result.close(); resolve(); };
  }));
  await other.goto(`${origin}/`, { waitUntil: 'networkidle' });
  const refused = await other.evaluate(() => __root('storageNotes').report().then(() => 'read', (error) => error.message));
  expect(refused.includes("keeps its rows by 'code'"), true, 'a changed Id field is refused, and no row is deleted');
  await changed.close();
}

async function checkWebAssemblyStorage(browser, origin) {
  const context = await browser.newContext();
  await context.addInitScript(rootHelper);
  const page = await context.newPage();
  await page.goto(origin, { waitUntil: 'networkidle' });
  const evaluate = (script) => page.evaluate(script);
  expect(JSON.stringify(await evaluate(() => [__root('goNotes').find('a'), __root('goNotes').find('a')])), JSON.stringify([{ key: 'a', reads: 1 }, { key: 'a', reads: 1 }]), 'a Go WebAssembly service reads its Cached value synchronously from memory');
  expect(JSON.stringify(await evaluate(() => __root('goNotes').find('b'))), JSON.stringify({ key: 'b', reads: 2 }), 'a Go WebAssembly service runs the method for a new key');
  await evaluate(() => __root('goNotes').save('g1', 'kept'));
  await evaluate(StorageGate.settleCaches);
  await page.reload({ waitUntil: 'networkidle' });
  expect(JSON.stringify(await evaluate(() => __root('goNotes').find('b'))), JSON.stringify({ key: 'b', reads: 2 }), 'the Go cache entry survives a reload through the Cache API');
  expect(JSON.stringify(await evaluate(() => __root('goNotes').list())), JSON.stringify([{ id: 'g1', text: 'kept' }]), 'the Go Store row survives a reload through the IndexedDB copy');
  expect(JSON.stringify(await evaluate(() => __root('goNotes').kept())), JSON.stringify([{ id: 'g1', text: 'kept' }]), 'a Go service that needs GoNoteStore reads through the store, which opens the context of its Setup');
  expect(await evaluate(() => __root('rsCounter').keptCount()), 0, 'a Rust service that needs RsNoteStore counts through the store, which opens the context of its Setup');
  expect(JSON.stringify(await evaluate(() => [__root('rubyNotes').find('a'), __root('rubyNotes').find('a')])), JSON.stringify([{ key: 'a', reads: 1 }, { key: 'a', reads: 1 }]), 'a Ruby WebAssembly service reads its Cached value synchronously from memory');
  await evaluate(() => __root('rubyNotes').forget('a'));
  expect(JSON.stringify(await evaluate(() => __root('rubyNotes').find('a'))), JSON.stringify({ key: 'a', reads: 2 }), 'ClearCache makes the Ruby service read again');
  await evaluate(() => __root('rubyNotes').save('r1', 'kept'));
  const refused = await evaluate(() => { try { __root('rubyNotes').saveThenRefuse('r2'); return ''; } catch (error) { return error.message; } });
  expect(refused.includes('the note was refused'), true, 'a refused Ruby AllOrNothing call reports its error');
  await evaluate(StorageGate.settleCaches);
  await page.reload({ waitUntil: 'networkidle' });
  expect(JSON.stringify(await evaluate(() => __root('rubyNotes').find('a'))), JSON.stringify({ key: 'a', reads: 2 }), 'the Ruby cache entry survives a reload through the Cache API');
  expect(JSON.stringify(await evaluate(() => __root('rubyNotes').list())), JSON.stringify([{ id: 'r1', text: 'kept' }]), 'the Ruby Store row survives a reload and the refused AllOrNothing saved nothing');
  expect(JSON.stringify(await evaluate(() => __root('rubyNotes').kept())), JSON.stringify([{ id: 'r1', text: 'kept' }]), 'a Ruby service that needs RubyNoteStore reads through the store, which opens the context of its Setup');
  if (StorageGate.withCSharp) await checkCSharpStorage(page, evaluate);
  if (StorageGate.withCSharp) await checkFsharpStorage(evaluate);
  if (StorageGate.withKotlin) await checkKotlinStorage(page, evaluate);
  if (StorageGate.withSwift) await checkSwiftStorage(page, evaluate);
  if (StorageGate.withJava) await checkJavaStorage(page, evaluate);
  await context.close();
}

async function checkWebAssemblyStoredFields(browser, origin) {
  const context = await browser.newContext();
  const page = await context.newPage();
  await page.goto(origin, { waitUntil: 'networkidle' });
  const services = [['rs', 'rsCounter', 'Rust'], ['go', 'goNotes', 'Go'], ['ruby', 'rubyNotes', 'Ruby']];
  if (StorageGate.withCSharp) services.push(['cs', 'csNotes', 'C#'], ['journal', 'journal', 'F#']);
  if (StorageGate.withKotlin) services.push(['kt', 'ktNotes', 'Kotlin']);
  if (StorageGate.withSwift) services.push(['swift', 'swiftCounter', 'Swift']);
  if (StorageGate.withJava) services.push(['java', 'javaCounter', 'Java']);
  for (const [id, , language] of services) {
    for (let click = 0; click < 3; click += 1) await page.click(`#${id}-bump`);
    await page.waitForFunction((selector) => document.querySelector(selector)?.textContent === '3', `#${id}-count`);
    expect(await page.textContent(`#${id}-count`), '3', `a ${language} WebAssembly method changes its LocalState field`);
  }
  await page.reload({ waitUntil: 'networkidle' });
  for (const [id, instance, language] of services) {
    expect(await page.textContent(`#${id}-count`), '3', `a ${language} LocalState field changed by its own method survives a reload`);
    const stored = await page.evaluate((key) => localStorage.getItem(key), `tac:service:${instance}:/:count`);
    expect(stored, '3', `a ${language} LocalState field changed by its own method is written to localStorage`);
  }
  await context.close();
}

async function checkCSharpStorage(page, evaluate) {
  expect(JSON.stringify(await evaluate(async () => [await __root('csNotes').find('a'), await __root('csNotes').find('a')])), JSON.stringify([{ key: 'a', reads: 1 }, { key: 'a', reads: 1 }]), 'a C# WebAssembly service reads its Cached value from the page cache');
  await evaluate(() => __root('csNotes').forget('a'));
  expect(JSON.stringify(await evaluate(() => __root('csNotes').find('a'))), JSON.stringify({ key: 'a', reads: 2 }), 'ClearCache makes the C# service read again');
  await evaluate(() => __root('csNotes').save('c1', 'kept'));
  const refused = await evaluate(async () => { try { await __root('csNotes').refuse('c2'); return false; } catch { return true; } });
  expect(refused, true, 'a refused C# AllOrNothing call fails');
  await evaluate(StorageGate.settleCaches);
  await page.reload({ waitUntil: 'networkidle' });
  expect(JSON.stringify(await evaluate(() => __root('csNotes').find('a'))), JSON.stringify({ key: 'a', reads: 2 }), 'the C# cache entry survives a reload through the Cache API');
  expect(JSON.stringify(await evaluate(() => __root('csNotes').list())), JSON.stringify([{ id: 'c1', text: 'kept' }]), 'the C# Store row survives a reload and the refused AllOrNothing saved nothing');
  expect(JSON.stringify(await evaluate(() => __root('csNotes').kept())), JSON.stringify([{ id: 'c1', text: 'kept' }]), 'a C# service that needs CsNoteStore reads through the store, which opens the context of its Setup');
}

async function checkFsharpStorage(evaluate) {
  await evaluate(() => __root('journal').keep('f1', 'kept'));
  expect(JSON.stringify(await evaluate(() => __root('journal').kept())), JSON.stringify([{ id: 'f1', text: 'kept' }]), 'an F# service whose constructor takes JournalStore saves and reads through the store, which extends Repository and opens the context of its Setup');
}

async function checkKotlinStorage(page, evaluate) {
  expect(JSON.stringify(await evaluate(() => [__root('ktNotes').find('a'), __root('ktNotes').find('a')])), JSON.stringify([{ key: 'a', reads: 1 }, { key: 'a', reads: 1 }]), 'a Kotlin WebAssembly service reads its Cached value synchronously from memory');
  await evaluate(() => __root('ktNotes').forget('a'));
  expect(JSON.stringify(await evaluate(() => __root('ktNotes').find('a'))), JSON.stringify({ key: 'a', reads: 2 }), 'ClearCache makes the Kotlin service read again');
  await evaluate(() => __root('ktNotes').save('k1', 'kept'));
  const refused = await evaluate(() => { try { __root('ktNotes').saveThenRefuse('k2'); return ''; } catch (error) { return error.message; } });
  expect(refused.includes('the note was refused'), true, 'a refused Kotlin AllOrNothing call reports its error');
  await evaluate(StorageGate.settleCaches);
  await page.reload({ waitUntil: 'networkidle' });
  expect(JSON.stringify(await evaluate(() => __root('ktNotes').find('a'))), JSON.stringify({ key: 'a', reads: 2 }), 'the Kotlin cache entry survives a reload through the Cache API');
  expect(JSON.stringify(await evaluate(() => __root('ktNotes').list())), JSON.stringify([{ id: 'k1', text: 'kept' }]), 'the Kotlin Store row survives a reload and the refused AllOrNothing saved nothing');
  expect(JSON.stringify(await evaluate(() => __root('ktNotes').kept())), JSON.stringify([{ id: 'k1', text: 'kept' }]), 'a Kotlin service that needs KtNoteStore reads through the store, which opens the context of its Setup');
}

async function checkSwiftStorage(page, evaluate) {
  expect(JSON.stringify(await evaluate(() => [__root('swiftCounter').find('a'), __root('swiftCounter').find('a')])), JSON.stringify([{ key: 'a', reads: 1 }, { key: 'a', reads: 1 }]), 'a Swift WebAssembly service reads its Cached value synchronously from memory');
  await evaluate(() => __root('swiftCounter').forget('a'));
  expect(JSON.stringify(await evaluate(() => __root('swiftCounter').find('a'))), JSON.stringify({ key: 'a', reads: 2 }), 'ClearCache makes the Swift service read again');
  await evaluate(() => __root('swiftCounter').save('s1', 'kept'));
  const refused = await evaluate(() => { try { __root('swiftCounter').saveThenRefuse('s2'); return ''; } catch (error) { return error.message; } });
  expect(refused.includes('the note was refused'), true, 'a refused Swift AllOrNothing call reports its error');
  await evaluate(StorageGate.settleCaches);
  await page.reload({ waitUntil: 'networkidle' });
  expect(JSON.stringify(await evaluate(() => __root('swiftCounter').find('a'))), JSON.stringify({ key: 'a', reads: 2 }), 'the Swift cache entry survives a reload through the Cache API');
  expect(JSON.stringify(await evaluate(() => __root('swiftCounter').list())), JSON.stringify([{ id: 's1', text: 'kept' }]), 'the Swift Store row survives a reload and the refused AllOrNothing saved nothing');
  expect(JSON.stringify(await evaluate(() => __root('swiftCounter').kept())), JSON.stringify([{ id: 's1', text: 'kept' }]), 'a Swift service that needs SwiftNoteStore reads through the store, which opens the context of its Setup');
}

async function checkJavaStorage(page, evaluate) {
  expect(JSON.stringify(await evaluate(() => [__root('javaCounter').find('a'), __root('javaCounter').find('a')])), JSON.stringify([{ key: 'a', reads: 1 }, { key: 'a', reads: 1 }]), 'a Java WebAssembly service reads its Cached value synchronously from memory');
  await evaluate(() => __root('javaCounter').forget('a'));
  expect(JSON.stringify(await evaluate(() => __root('javaCounter').find('a'))), JSON.stringify({ key: 'a', reads: 2 }), 'ClearCache makes the Java service read again');
  await evaluate(() => __root('javaCounter').save('j1', 'kept'));
  const refused = await evaluate(() => { try { __root('javaCounter').saveThenRefuse('j2'); return ''; } catch (error) { return error.message; } });
  expect(refused.includes('the note was refused'), true, 'a refused Java AllOrNothing call reports its error');
  await evaluate(StorageGate.settleCaches);
  await page.reload({ waitUntil: 'networkidle' });
  expect(JSON.stringify(await evaluate(() => __root('javaCounter').find('a'))), JSON.stringify({ key: 'a', reads: 2 }), 'the Java cache entry survives a reload through the Cache API');
  expect(JSON.stringify(await evaluate(() => __root('javaCounter').list())), JSON.stringify([{ id: 'j1', text: 'kept' }]), 'the Java Store row survives a reload and the refused AllOrNothing saved nothing');
  expect(JSON.stringify(await evaluate(() => __root('javaCounter').kept())), JSON.stringify([{ id: 'j1', text: 'kept' }]), 'a Java service that needs JavaNoteStore reads through the store, which opens the context of its Setup');
}

async function checkInjectedClients(page) {
  const binding = await page.evaluate(() => ({
    client: typeof __root('storageClients').cacheFirstClient?.counter === 'function',
    fetchRemoved: !('tac' in __root('storageClients')),
    hidden: !Object.keys(__root('storageClients')).includes('tac') && !Object.keys(__root('storageClients')).includes('cacheFirstClient'),
    clonable: (() => { try { structuredClone({...__root('storageClients')}); return true; } catch { return false; } })(),
  }));
  expect(binding.client, true, 'service receives an injected Client');
  expect(binding.fetchRemoved, true, 'public runtime has no fetch method');
  expect(binding.hidden, true, 'runtime and injected dependencies are not enumerable');
  expect(binding.clonable, true, 'owner state clones for hot update');

  const announce = async (size) => page.evaluate(async (size) => {
    __root('storageClients').announce(size);
    await new Promise((resolve) => setTimeout(resolve, 50));
    return __root('storageClients').label;
  }, size);
  expect(await announce(3), 'clients|first:3|warmed:3|async:3', 'injected publisher, Order, typed listeners, Async and returned model');
  expect(await announce(1), 'clients|first:1|async:1', 'condition skips only the listener it guards');
}

async function checkCachePolicies(page, origin, context) {
  const api = `${origin}/api/counter`;
  await page.evaluate((url) => new Promise((resolve, reject) => {
    const opened = indexedDB.open('tachyon-fetch-cache', 1);
    opened.onupgradeneeded = () => opened.result.createObjectStore('responses', {keyPath:'key'});
    opened.onerror = () => reject(opened.error);
    opened.onsuccess = () => {
      const transaction = opened.result.transaction('responses','readwrite');
      transaction.objectStore('responses').put({key:`fetch:GET:${url}`, status:200,
        headers:[['content-type','application/json']], updatedAt:Date.now(),
        body:new TextEncoder().encode(JSON.stringify({value:-999})).buffer});
      transaction.oncomplete = () => { opened.result.close(); resolve(); };
      transaction.onerror = () => { opened.result.close(); reject(transaction.error); };
    };
  }), api);
  const first = await page.evaluate(() => __root('storageClients').cacheFirstClient.counter());
  const repeat = await page.evaluate(() => __root('storageClients').cacheFirstClient.counter());
  expect(first.value !== -999, true, 'legacy credentialed cache discarded on privacy migration');
  expect(repeat.value, first.value, 'cache-first returns stored JSON');
  const fresh = await page.evaluate(() => __root('storageClients').networkFirstClient.counter());
  expect(fresh.value > first.value, true, 'network-first requests current JSON');
  const reloaded = await page.evaluate(() => __root('storageClients').reloadClient.counter());
  expect(reloaded.value > fresh.value, true, 'reload requires network and refreshes persistence');
  const unstored = await page.evaluate(() => __root('storageClients').freshClient.counter());
  const stillStored = await page.evaluate(() => __root('storageClients').cacheFirstClient.counter());
  expect(unstored.value > reloaded.value, true, 'no-store requests network');
  expect(stillStored.value, reloaded.value, 'no-store leaves the stored response unchanged');

  await page.evaluate((url) => __root('storageClients').cacheManager.invalidate([`fetch:GET:${url}`]), api);
  const afterInvalidate = await page.evaluate(() => __root('storageClients').cacheFirstClient.counter());
  expect(afterInvalidate.value > unstored.value, true, 'explicit invalidation forces a new read');
  await context.addCookies([{name:'private-session',value:'test-only',url:origin}]);
  const privacy = await page.evaluate(async () => {
    const client = __root('storageClients').cacheFirstClient;
    const bypassed = async (read) => {
      const first = await read();
      const second = await read();
      return second.value > first.value;
    };
    return {
      cookiesOmitted: !(await client.counterAsking('yes')).cookie,
      authorization: await bypassed(() => client.counterSigned('yes', 'test-fixture')),
      noStore: await bypassed(() => client.counterWithPolicy('no-store')),
      private: await bypassed(() => client.counterWithPolicy('private')),
    };
  });
  for (const [name, allowed] of Object.entries(privacy)) expect(allowed, true, `${name} preserves cache privacy`);
  return afterInvalidate;
}

async function checkRangeRequests(page) {
  const range = await page.evaluate(async () => {
    const client = __root('storageClients').cacheFirstClient;
    const partial = await client.rangePart('partial-first', 'bytes=0-2');
    const full = await client.range('partial-first');
    const warmed = await client.range('full-first');
    const rangedAfterFull = await client.rangePart('full-first', 'bytes=0-2');
    const stillFull = await client.range('full-first');
    const ifRange = await client.rangeIf('full-first', '"fixture-etag"');
    const unsolicited = await client.rangeUnasked('yes');
    const repeatedPartial = await client.rangeUnasked('yes');
    const mislabelled = await client.rangeMislabelled('yes', 'yes');
    const repeatedMislabelled = await client.rangeMislabelled('yes', 'yes');
    return {partial,full,warmed,rangedAfterFull,stillFull,ifRange,unsolicited,repeatedPartial,mislabelled,repeatedMislabelled};
  });
  expect(range.partial.status, 206, 'Range receives a partial network response');
  expect(range.full.status, 200, 'partial JSON cannot poison a full read');
  expect(range.full.body, 'abcdef', 'full read receives complete JSON');
  expect(range.rangedAfterFull.status, 206, 'Range bypasses existing full cache');
  expect(range.stillFull.number, range.warmed.number, 'Range preserves full cached representation');
  expect(range.ifRange.number !== range.warmed.number, true, 'If-Range bypasses cached full response');
  expect(range.repeatedPartial.number !== range.unsolicited.number, true, 'unsolicited 206 responses are not cached');
  expect(range.repeatedMislabelled.number !== range.mislabelled.number, true, 'Content-Range responses are not cached');
}

async function checkLegacyPartialResponses(page, origin) {
  for (const status of [200,206]) {
    const restored = await page.evaluate(async ({origin,status}) => {
      const url = `${origin}/api/range?legacy=${status}`;
      await new Promise((resolve,reject) => {
        const opened = indexedDB.open('tachyon-fetch-cache');
        opened.onerror = () => reject(opened.error);
        opened.onsuccess = () => {
          const transaction = opened.result.transaction('responses','readwrite');
          transaction.objectStore('responses').put({key:`fetch:GET:${url}`,status,
            headers:status===200?{'content-range':'bytes 0-2/6'}:{},
            updatedAt:Date.now(),body:new TextEncoder().encode('abc').buffer});
          transaction.oncomplete = () => { opened.result.close(); resolve(); };
          transaction.onerror = () => { opened.result.close(); reject(transaction.error); };
        };
      });
      return __root('storageClients').cacheFirstClient.rangeLegacy(String(status));
    },{origin,status});
    expect(restored.body,'abcdef',`legacy partial response (${status}) rejected`);
  }
}

async function checkOfflineBehavior(page, context, afterInvalidate) {
  await context.setOffline(true);
  const offline = await page.evaluate(() => __root('storageClients').networkFirstClient.counter());
  expect(offline.value, afterInvalidate.value, 'network-first falls back to IndexedDB offline');
  const refused = await page.evaluate(async () => {
    try { await __root('storageClients').freshClient.counter(); return false; } catch { return true; }
  });
  expect(refused,true,'no-store never answers from persistence');
  const offlineRange = await page.evaluate(async () => {
    try { await __root('storageClients').cacheFirstClient.rangePart('full-first', 'bytes=0-2'); return false; }
    catch { return true; }
  });
  expect(offlineRange,true,'offline Range never falls back to complete cached JSON');
  await context.setOffline(false);
  const aborted = await page.evaluate(async () => {
    dispatchEvent(new PageTransitionEvent('pagehide',{persisted:false}));
    try { await __root('storageClients').cacheFirstClient.counter(); return false; } catch { return true; }
  });
  expect(aborted,true,'disposed services reject instead of reading cached JSON');
}

async function checkDeniedStorage(browser, origin) {
  const blocked = await browser.newContext();
  await blocked.addInitScript(rootHelper);
  await blocked.addInitScript(() => {
    for (const name of ['localStorage', 'sessionStorage', 'indexedDB', 'caches']) {
      Object.defineProperty(globalThis, name, { get() { throw new DOMException('Denied', 'SecurityError'); } });
    }
  });
  const deniedPage = await blocked.newPage();
  await deniedPage.goto(origin, { waitUntil: 'networkidle' });
  await deniedPage.click('#bump');
  expect(await deniedPage.textContent('#clicks'), '1', 'denied Web Storage does not break rendering');
  const deniedClient = await deniedPage.evaluate(() => __root('storageClients').cacheFirstClient.counter());
  expect(deniedClient.value > 0, true, 'denied IndexedDB degrades to successful Client network response');
  await blocked.close();
}

function issueCertificate(project) {
  const key = path.join(project, 'ty-storage-key.pem');
  const certificate = path.join(project, 'ty-storage-cert.pem');
  const issued = spawnSync('openssl', [
    'req', '-x509', '-newkey', 'rsa:2048', '-nodes',
    '-keyout', key, '-out', certificate, '-days', '1',
    '-subj', `/CN=${StorageGate.host}`, '-addext', `subjectAltName=DNS:${StorageGate.host}`,
  ], { encoding: 'utf8', timeout: 30_000, killSignal: 'SIGKILL', maxBuffer: 1024 * 1024 });
  if (issued.status !== 0) {
    throw new Error(`cannot issue a test certificate: ${issued.error?.message || issued.stderr || issued.stdout}`);
  }
  return { key, certificate };
}

async function checkServiceWorkerOffline(secureContext, secureOrigin) {
  const securePage = await secureContext.newPage();
  await securePage.goto(`${secureOrigin}/`, { waitUntil: 'networkidle' });
  await securePage.evaluate(async () => {
    let timer;
    try {
      await Promise.race([
        navigator.serviceWorker.ready,
        new Promise((_, reject) => { timer = setTimeout(() => reject(new Error('Service worker readiness timed out')), 15_000); }),
      ]);
    } finally { clearTimeout(timer); }
  });
  await securePage.reload({ waitUntil: 'networkidle' });

  const detail = '/.tachyon/services/PanelDetailService.js';
  expect(await securePage.textContent('#clicks'), '0', 'fresh profile starts at the default');
  expect(await securePage.locator('#detail').count(), 0, 'branch is untaken, so the module is unused');

  const cached = await securePage.waitForFunction(async (asset) => {
    const names = await caches.keys();
    const name = names.find((value) => value.startsWith('tachyon-static-'));
    if (!name) return false;
    const cache = await caches.open(name);
    return Boolean(await cache.match(new URL(asset, location.href).href));
  }, detail).then(() => true).catch(() => false);
  expect(cached, true, 'unused service module precached at idle');

  await secureContext.setOffline(true);
  await securePage.click('#bump');
  await securePage.click('#bump');
  await securePage.click('#bump');
  expect(await securePage.textContent('#detail'), 'detail', 'precached component rendered offline');
}

async function checkBrowser(browser, name, origin, port) {
  const context = await browser.newContext();
  await context.addInitScript(rootHelper);
  context.setDefaultTimeout(15_000);
  context.setDefaultNavigationTimeout(15_000);
  const page = await checkFieldStorage(context, origin);
  await checkKeptCaches(page);
  if (name === 'chromium') await checkInsecureCaches(browser, port);
  await checkIndexedStore(browser, origin);
  await checkWebAssemblyStorage(browser, origin);
  await checkWebAssemblyStoredFields(browser, origin);
  await checkInjectedClients(page);
  const afterInvalidate = await checkCachePolicies(page, origin, context);
  await checkRangeRequests(page);
  await checkLegacyPartialResponses(page, origin);
  await checkOfflineBehavior(page, context, afterInvalidate);
  await checkDeniedStorage(browser, origin);
}

async function main() {
  if (!StorageGate.withCSharp) {
    if (process.env.CI) fail('CI must provide .NET wasm-tools for the C# storage check');
    console.log('skipping the C# storage check: the .NET wasm-tools workload is not installed');
  }
  if (!StorageGate.withKotlin) console.log('skipping the Kotlin storage check: kotlinc-js or TAC_KOTLIN_WASM_STDLIB is missing');
  if (!StorageGate.withSwift) console.log('skipping the Swift stored-field check: TAC_SWIFT_WASM_SDK is not set');
  if (!StorageGate.withJava) console.log('skipping the Java stored-field check: TAC_TEAVM_HOME is not set');
  let project;
  let server;
  let browser;
  let secure;
  let secureContext;
  try {
    project = mkdtempSync(path.join(tmpdir(), 'ty-storage-gate-'));
    writeFixture(project);
    buildFixture(project);
    const handle = createHandler(project);
    server = createServer(handle);
    await listen(server);
    const origin = `http://127.0.0.1:${server.address().port}`;

    for (const name of StorageGate.browsers) {
      console.log(`storage checks in ${name}`);
      browser = await StorageGate.open(name, project);
      await checkBrowser(browser, name, origin, server.address().port);
      await browser.close();
      browser = undefined;
    }
    await closeServer(server);
    server = undefined;

    const { key, certificate } = issueCertificate(project);
    secure = createSecureServer({ key: readFileSync(key), cert: readFileSync(certificate) }, handle);
    await listen(secure);
    const secureOrigin = `https://${StorageGate.host}:${secure.address().port}`;
    const profile = path.join(project, 'ty-storage-profile');
    secureContext = await chromium.launchPersistentContext(profile, {
      timeout: 30_000,
      ignoreHTTPSErrors: true,
      args: [StorageGate.hostRule, '--ignore-certificate-errors'],
    });
    secureContext.setDefaultTimeout(15_000);
    secureContext.setDefaultNavigationTimeout(15_000);
    await checkServiceWorkerOffline(secureContext, secureOrigin);
  } finally {
    try {
      const cleanup = await Promise.allSettled([
        browser?.close(), secureContext?.close(), closeServer(server), closeServer(secure),
      ]);
      for (const result of cleanup) {
        if (result.status === 'rejected') { console.error('storage cleanup failed:', result.reason); process.exitCode = 1; }
      }
    } finally {
      if (project) rmSync(project, { recursive: true, force: true });
    }
  }

  if (process.exitCode) console.error('storage gate failed');
  else console.log('storage gate passed');
}

await main();
