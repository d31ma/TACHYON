#!/usr/bin/env node

import { CargoArtifacts } from './cargo-artifacts.mjs'
import { createHash } from 'node:crypto'
import { chromium, firefox, webkit } from 'playwright'
import { createServer } from 'node:http'
import { spawnSync } from 'node:child_process'
import { createReadStream, existsSync, mkdtempSync, mkdirSync, rmSync, statSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import process from 'node:process'

class CachedClientGate {
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('release/ty')
  static browsers = (process.env.TAC_CACHED_CLIENT_BROWSERS ?? 'chromium,firefox,webkit').split(',')
  static launchers = { chromium, firefox, webkit }
  static mimeTypes = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.wasm': 'application/wasm' }
  static withJava = Boolean(process.env.TAC_TEAVM_HOME) && existsSync(process.env.TAC_TEAVM_HOME)
  static languages = process.env.TAC_CACHED_CLIENT_LANGUAGES?.split(',') ?? ['js', 'ts', 'go', 'kt', 'dart', 'swift', 'fs', 'rs', 'cs', 'rb', ...(CachedClientGate.withJava ? ['java'] : [])]
  static settleTimeout = 10_000
  static settleStep = 50
  static publicAnswer = 'public, max-age=600'
  static publicStamp = 0
  static settings = '{"name":"stamps","ttlMs":60000,"maxBytes":4096,"maxEntries":8}'

  static files = {
    'client/manifest.json': '{"tac":{"names":{"words":["js","ts","kt","fs","rs","cs","rb"]},"visibility":{"check":false},"budgets":{"webassembly":6144}}}\n',
    'client/pages/tac.html': '<stamp-page />\n',
    'client/components/stamp/page/tac.html': `<main>${CachedClientGate.languages.map((language) => `<p id="${language}">{${language}Stamp.stamp}</p>`).join('\n')}</main>\n`,
    'client/remotes/JsStampClient.js': `@HTTPClient
@CacheSettings(${CachedClientGate.settings})
export default class JsStampClient {
  /** @returns {Promise<BehaviorJsonValue>} */
  @Cached
  @GetRequest("/api/stamp")
  async read() {}
}
`,
    'client/remotes/JsPublicStampClient.js': `@HTTPClient
@CacheSettings(${CachedClientGate.settings})
export default class JsPublicStampClient {
  /** @returns {Promise<BehaviorJsonValue>} */
  @Cached
  @GetRequest("/api/public-stamp")
  async read() {}

  /** @returns {Promise<BehaviorJsonValue>} */
  @ClearCache({"method":"read"})
  @PostRequest("/api/public-stamp/clear")
  async clear() {}
}
`,
    'client/services/JsStampService.js': `@Service
export default class JsStampService {
  /** @type {JsStampClient} */
  @Needs
  jsStampClient;
  /** @type {JsPublicStampClient} */
  @Needs
  jsPublicStampClient;
  stamp = '';
  loads = 0;

  async load() {
    this.stamp = JSON.stringify(await this.jsStampClient.read());
    this.loads += 1;
  }

  async readPublic() {
    return JSON.stringify(await this.jsPublicStampClient.read());
  }

  async clearPublic() {
    await this.jsPublicStampClient.clear();
  }
}
`,
    'client/remotes/TsStampClient.ts': `@HTTPClient
@CacheSettings(${CachedClientGate.settings})
export default class TsStampClient {
  @Cached
  @GetRequest("/api/stamp")
  async read(): Promise<BehaviorJsonValue> {}
}
`,
    'client/services/TsStampService.ts': `@Service
export default class TsStampService {
  @Needs
  readonly tsStampClient!: TsStampClient;
  stamp = '';
  loads = 0;

  async load(): Promise<void> {
    this.stamp = JSON.stringify(await this.tsStampClient.read());
    this.loads += 1;
  }
}
`,
    'client/remotes/GoStampClient.go': `// @CacheSettings(${CachedClientGate.settings})
// @HTTPClient
type GoStampClient interface {
	// @Cached
	// @GetRequest("/api/stamp")
	Read() (BehaviorValue, error)
}
`,
    'client/setups/GoStampSetup.go': `import "math/rand"

// @Setup
type GoStampSetup struct{}

func (GoStampSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	config := NewBehaviorConfiguration()
	config.Http["goStamp"] = GoStampSetup{}
	return config, nil
}

func (GoStampSetup) Exchange(request *BehaviorHttpRequest, execution *BehaviorExecution) (BehaviorValue, error) {
	return map[string]any{"stamp": rand.Intn(1_000_000_000) + 1}, nil
}
`,
    'client/services/GoStampService.go': `import "fmt"

// @Service
type GoStampService struct {
	stamps *GoStampClient
	Stamp  string
	Loads  int
}

// @OnStart
func (service *GoStampService) Start() error {
	client, err := NewGoStampClient()
	service.stamps = client
	return err
}

func (service *GoStampService) Load() error {
	answer, err := service.stamps.Read()
	service.Stamp = fmt.Sprint(answer)
	service.Loads += 1
	return err
}
`,
    'client/remotes/KtStampClient.kt': `@CacheSettings(${CachedClientGate.settings})
@HTTPClient
interface KtStampClient {
    @Cached
    @GetRequest("/api/stamp")
    fun read(): Any?
}
`,
    'client/setups/KtStampSetup.kt': `@Setup
class KtStampSetup {
    companion object {
        fun configure(execution: Behaviors.Execution): Behaviors.Configuration =
            Behaviors.Configuration(http = mapOf("ktStamp" to Behaviors.HttpAdapter { _, _ -> mapOf("stamp" to kotlin.random.Random.nextInt(1, 1_000_000_000)) }))
    }
}
`,
    'client/services/KtStampService.kt': `@Service
class KtStampService {
    private val stamps: KtStampClient = KtStampClient()
    var stamp: String = ""
    var loads: Int = 0

    fun load() {
        stamp = stamps.read().toString()
        loads += 1
    }
}
`,
    'client/remotes/DartStampClient.dart': `@CacheSettings(${CachedClientGate.settings})
@HTTPClient
abstract interface class DartStampClient {
  @Cached
  @GetRequest("/api/stamp")
  Future<Object?> read();
}
`,
    'client/setups/DartStampSetup.dart': `import 'dart:math';

@Setup
class DartStampSetup {
  static BehaviorAdapters configure(BehaviorExecution execution) {
    return BehaviorAdapters(
      http: {
        'dartStamp': BehaviorHttpAdapter(exchange: (request, execution) async => {
              'stamp': Random().nextInt(1000000000) + 1,
            }),
      },
    );
  }
}
`,
    'client/services/DartStampService.dart': `@Service
class DartStampService {
  final DartStampClient stamps = DartStampClient();
  String stamp = '';
  int loads = 0;

  Future<void> load() async {
    stamp = (await stamps.read()).toString();
    loads += 1;
  }
}
`,
    'client/remotes/SwiftStampClient.swift': `@CacheSettings(${CachedClientGate.settings})
@HTTPClient
protocol SwiftStampClient {
    @Cached
    @GetRequest("/api/stamp")
    func read() async throws -> BehaviorJSON
}
`,
    'client/setups/SwiftStampSetup.swift': `@Setup
final class SwiftStampSetup {
    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters {
        var adapters = BehaviorAdapters()
        adapters.http["swiftStamp"] = BehaviorHTTPAdapter { request, execution in
            return try BehaviorJSON.parse("{\\"stamp\\":\\(Int.random(in: 1...1_000_000_000))}")
        }
        return adapters
    }
}
`,
    'client/services/SwiftStampService.swift': `@Service
final class SwiftStampService {
    let stamps: SwiftStampClient = SwiftStampClient()
    var stamp: String = ""
    var loads: Int = 0

    func load() async throws {
        stamp = String(describing: try await stamps.read())
        loads += 1
    }
}
`,
    'client/remotes/FsStampClient.fs': `open System.Threading.Tasks

[<CacheSettings(${CachedClientGate.settings})>]
[<HTTPClient>]
type FsStampClient =
    [<Cached>]
    [<GetRequest("/api/stamp")>]
    abstract Read: unit -> Task<BehaviorJSON>
`,
    'client/setups/FsStampSetup.fs': `open System.Threading.Tasks

[<Setup>]
type FsStampSetup() =
    static member configure(execution: BehaviorExecution) : BehaviorAdapters =
        let adapters = BehaviorAdapters()
        adapters.http["fsStamp"] <-
            { new BehaviorHTTPAdapter with
                member this.exchange(request: BehaviorJSON, execution: BehaviorExecution) : Task<BehaviorJSON> =
                    Task.FromResult(BehaviorJSON.Parse(sprintf "{\\"stamp\\":%d}" (System.Random.Shared.Next(1, 1000000000)))) }
        adapters
`,
    'client/services/FsStampService.fs': `open System.Threading.Tasks

[<Service>]
type FsStampService() =
    let stamps = FsStampClient()
    member val stamp : string = "" with get, set
    member val loads : int = 0 with get, set

    member this.Load() : Task =
        task {
            let! answer = stamps.Read()
            this.stamp <- answer.ToString()
            this.loads <- this.loads + 1
        }
`,
    'client/remotes/RsStampClient.rs': `#[CacheSettings(${CachedClientGate.settings})]
#[HTTPClient]
trait RsStampClient {
    #[Cached]
    #[GetRequest("/api/stamp")]
    fn read(&self) -> BehaviorResult<BehaviorValue>;
}
`,
    'client/setups/RsStampSetup.rs': `#[tac::setup]
struct RsStampSetup {}
impl RsStampSetup {
    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {
        struct StampTransport {
            calls: std::sync::atomic::AtomicU32,
        }
        impl BehaviorHttpAdapter for StampTransport {
            fn exchange(&self, _request: &BehaviorHttpRequest, _execution: &BehaviorExecution) -> BehaviorResult<BehaviorValue> {
                let stamp = self.calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                Ok(BehaviorValue::object([("stamp", BehaviorValue::Number(f64::from(stamp)))]))
            }
        }

        let mut configuration = BehaviorConfiguration::default();
        let transport = StampTransport { calls: std::sync::atomic::AtomicU32::new(0) };
        configuration.http.insert("rsStamp".into(), std::sync::Arc::new(transport));
        Ok(configuration)
    }
}
`,
    'client/services/RsStampService.rs': `#[Service]
struct RsStampService { stamps: RsStampClient, stamp: String, loads: i64 }
impl RsStampService {
    fn new() -> Self {
        Self { stamps: RsStampClient::new(), stamp: String::new(), loads: 0 }
    }

    pub fn load(&mut self) {
        let answer = self.stamps.read().unwrap_or(BehaviorValue::Null);
        self.stamp = format!("{answer:?}");
        self.loads += 1;
    }
}
`,
    'client/remotes/CsStampClient.cs': `[CacheSettings(${CachedClientGate.settings})]
[HTTPClient]
public interface CsStampClient {
    [Cached]
    [GetRequest("/api/stamp")]
    System.Threading.Tasks.Task<BehaviorJSON> Read();
}
`,
    'client/setups/CsStampSetup.cs': `[Setup]
public class CsStampSetup {
    sealed class StampTransport : BehaviorHTTPAdapter {
        public System.Threading.Tasks.Task<BehaviorJSON> exchange(BehaviorJSON request, BehaviorExecution execution) {
            var stamp = System.Random.Shared.Next(1, 1000000000);
            return System.Threading.Tasks.Task.FromResult(BehaviorJSON.Parse("{\\"stamp\\":" + stamp + "}"));
        }
    }

    public static BehaviorAdapters configure(BehaviorExecution execution) {
        var adapters = new BehaviorAdapters();
        adapters.http["csStamp"] = new StampTransport();
        return adapters;
    }
}
`,
    'client/services/CsStampService.cs': `[Service]
public class CsStampService {
    readonly CsStampClient _stamps = new CsStampClient();
    public string stamp = "";
    public int loads = 0;

    public async System.Threading.Tasks.Task Load() {
        stamp = (await _stamps.Read()).ToString();
        loads += 1;
    }
}
`,
    'client/remotes/RbStampClient.rb': `#[CacheSettings(${CachedClientGate.settings})]
#[HTTPClient]
class RbStampClient
  # @return [Hash]
  #[Cached]
  #[GetRequest("/api/stamp")]
  def read
  end
end
`,
    'client/setups/RbStampSetup.rb': `#[Setup]
class RbStampSetup
  # @param _execution [BehaviorExecution]
  # @return [Hash]
  def self.configure(_execution)
    stamps = { "exchange" => ->(_request, _running) { { "stamp" => rand(1..1_000_000_000) } } }
    { "http" => { "rbStamp" => stamps } }
  end
end
`,
    'client/services/RbStampService.rb': `#[Service]
class RbStampService
  # @return [String]
  attr_reader :stamp
  # @return [Integer]
  attr_reader :loads

  def initialize
    @stamps = RbStampClient.new
    @stamp = ""
    @loads = 0
  end

  # @return [void]
  def load
    @stamp = @stamps.read.to_s
    @loads += 1
  end
end
`,
    ...(CachedClientGate.withJava ? { 'client/remotes/JavaStampClient.java': `@CacheSettings(${CachedClientGate.settings})
@HTTPClient
interface JavaStampClient {
    @Cached
    @GetRequest("/api/stamp")
    Object read();
}
`,
    'client/setups/JavaStampSetup.java': `import java.util.Map;

@Setup
class JavaStampSetup {
    static Behaviors.Configuration configure(Behaviors.Execution execution) {
        Behaviors.HttpAdapter stamps = (request, running) -> Map.of("stamp", new java.util.Random().nextInt(1_000_000_000) + 1);
        return new Behaviors.Configuration(Map.of(), Map.of("javaStamp", stamps), Map.of(), null);
    }
}
`,
    'client/services/JavaStampService.java': `@Service
class JavaStampService {
    private final JavaStampClient stamps = new JavaStampClient();
    String stamp = "";
    int loads = 0;

    void load() {
        stamp = String.valueOf(stamps.read());
        loads += 1;
    }
}
` } : {}),
  }

  static write(project) {
    for (const [relative, contents] of Object.entries(CachedClientGate.files)) {
      const language = path.extname(relative).slice(1)
      if (['clients', 'remotes', 'services', 'setups'].includes(relative.split('/')[1]) && !CachedClientGate.languages.includes(language)) continue
      const file = path.join(project, relative)
      mkdirSync(path.dirname(file), { recursive: true })
      writeFileSync(file, contents)
    }
  }

  static checkBudgetRefusal() {
    const project = mkdtempSync(path.join(tmpdir(), 'tachyon-budget-refusal-'))
    try {
      mkdirSync(path.join(project, 'client/shared/scripts'), { recursive: true })
      mkdirSync(path.join(project, 'client/pages'), { recursive: true })
      mkdirSync(path.join(project, 'client/components/budget/control'), { recursive: true })
      writeFileSync(path.join(project, 'client/manifest.json'), JSON.stringify({ tac: { scripts: ['/shared/scripts/BudgetPadding.js'] } }))
      writeFileSync(path.join(project, 'client/pages/tac.html'), '<budget-control />\n')
      writeFileSync(path.join(project, 'client/components/budget/control/tac.html'), '<main>Budget control</main>\n')
      const padding = Array.from({ length: 10_240 }, (_, index) => createHash('sha256').update(String(index)).digest('hex')).join('')
      writeFileSync(path.join(project, 'client/shared/scripts/BudgetPadding.js'), `export default class BudgetPadding { static _TEXT = '${padding}' }`)
      const built = spawnSync(CachedClientGate.binary, ['build', project], {
        encoding: 'utf8', timeout: 120_000, killSignal: 'SIGKILL', maxBuffer: 4 * 1024 * 1024,
      })
      const output = `${built.stdout ?? ''}${built.stderr ?? ''}`
      const diagnostics = output.match(/error\[TY[0-9]+\]/g) ?? []
      if (built.error || built.signal || built.status === null || built.status === 0
          || diagnostics.length !== 1 || diagnostics[0] !== 'error[TY1337]'
          || !output.includes("Route '/' loads") || !output.includes('budget is 200 KB')
          || !output.includes('BudgetPadding.js')) {
        throw new Error(`the oversized script must fail only the default route budget: ${built.error?.message || built.signal || output}`)
      }
      console.log('budget control: oversized script refused with TY1337 at the default 200 KB limit')
    } finally {
      rmSync(project, { recursive: true, force: true })
    }
  }

  static build(project) {
    const built = spawnSync(CachedClientGate.binary, ['build', project], {
      encoding: 'utf8', timeout: 600_000, killSignal: 'SIGKILL', maxBuffer: 4 * 1024 * 1024,
    })
    if (built.status !== 0) throw new Error(`cached client fixture build failed: ${built.error?.message || built.stderr || built.stdout}`)
  }

  static handler(project) {
    const root = path.join(project, 'dist')
    return (request, response) => {
      const url = new URL(request.url, 'http://localhost')
      if (url.pathname === '/api/stamp') {
        response.writeHead(200, { 'content-type': 'application/json', 'cache-control': 'no-store' })
        response.end(JSON.stringify({ stamp: Math.floor(Math.random() * 1_000_000_000) + 1 }))
        return
      }
      if (url.pathname === '/api/public-stamp') {
        CachedClientGate.publicStamp += 1
        response.writeHead(200, { 'content-type': 'application/json', 'cache-control': CachedClientGate.publicAnswer })
        response.end(JSON.stringify({ stamp: CachedClientGate.publicStamp }))
        return
      }
      if (url.pathname === '/api/public-stamp/clear') {
        response.writeHead(200, { 'content-type': 'application/json', 'cache-control': 'no-store' })
        response.end('{"cleared":true}')
        return
      }
      let file = path.join(root, decodeURIComponent(url.pathname))
      if (existsSync(file) && statSync(file).isDirectory()) file = path.join(file, 'index.html')
      if (!existsSync(file)) {
        response.writeHead(404).end()
        return
      }
      response.writeHead(200, { 'content-type': CachedClientGate.mimeTypes[path.extname(file)] ?? 'application/octet-stream' })
      createReadStream(file).pipe(response)
    }
  }

  static expect(actual, wanted, label) {
    if (actual !== wanted) throw new Error(`${label}: expected ${JSON.stringify(wanted)}, got ${JSON.stringify(actual)}`)
    console.log(`ok - ${label}`)
  }

  static read(page, language, member) {
    return page.evaluate(([name, key]) => __tachyonTac.service(`${name}Stamp`)[key], [language, member])
  }

  static async loaded(page, language, count) {
    await page.evaluate((name) => { void __tachyonTac.service(`${name}Stamp`).load() }, language)
    const deadline = Date.now() + CachedClientGate.settleTimeout
    while (await CachedClientGate.read(page, language, 'loads') !== count && Date.now() < deadline) {
      await page.waitForTimeout(CachedClientGate.settleStep)
    }
    return CachedClientGate.read(page, language, 'stamp')
  }

  static async check(page, browser, language) {
    const first = await CachedClientGate.loaded(page, language, 1)
    CachedClientGate.expect(first.includes('stamp'), true, `${browser} ${language}: an HTTPClient read answers`)
    const second = await CachedClientGate.loaded(page, language, 2)
    CachedClientGate.expect(second, first, `${browser} ${language}: a Cached HTTPClient read answers from its cache`)
  }

  static async checkClearCache(page, browser) {
    const read = () => page.evaluate(() => __tachyonTac.service('jsStamp').readPublic())
    const first = await read()
    CachedClientGate.expect(await read(), first, `${browser} js: a Cached read of a public answer answers from its cache`)
    await page.evaluate(() => __tachyonTac.service('jsStamp').clearPublic())
    const after = await read()
    CachedClientGate.expect(after !== first, true, `${browser} js: after ClearCache the next read skips the browser HTTP cache (${first} then ${after})`)
  }

  static async run(project, origin, name) {
    const browser = await CachedClientGate.launchers[name].launch({ timeout: 30_000 })
    try {
      const page = await browser.newPage()
      await page.goto(`${origin}/`, { waitUntil: 'networkidle' })
      for (const language of CachedClientGate.languages) await CachedClientGate.check(page, name, language)
      if (CachedClientGate.languages.includes('js')) await CachedClientGate.checkClearCache(page, name)
    } finally {
      await browser.close()
    }
  }

  static async main() {
    if (process.env.TAC_BROWSER_REQUIRE_TOOLCHAINS && !CachedClientGate.withJava) throw new Error('TAC_BROWSER_REQUIRE_TOOLCHAINS is set, but TAC_TEAVM_HOME does not name the TeaVM jars for Java')
    const project = mkdtempSync(path.join(tmpdir(), 'tachyon-cached-client-'))
    const server = createServer(CachedClientGate.handler(project))
    try {
      CachedClientGate.write(project)
      CachedClientGate.checkBudgetRefusal()
      CachedClientGate.build(project)
      await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve))
      const origin = `http://127.0.0.1:${server.address().port}`
      for (const name of CachedClientGate.browsers) await CachedClientGate.run(project, origin, name)
    } finally {
      server.close()
      rmSync(project, { recursive: true, force: true })
    }
  }
}

await CachedClientGate.main()
