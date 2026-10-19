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

class WindowControlsGate {
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('release/ty')
  static browsers = (process.env.TAC_WINDOW_BROWSERS ?? 'chromium,firefox,webkit').split(',')
  static launchers = { chromium, firefox, webkit }
  static mimeTypes = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.wasm': 'application/wasm' }
  static withJava = Boolean(process.env.TAC_TEAVM_HOME) && existsSync(process.env.TAC_TEAVM_HOME)
  static allLanguages = ['js', 'ts', 'rust', 'go', 'csharp', 'fsharp', 'kotlin', 'dart', 'ruby', 'swift', ...(WindowControlsGate.withJava ? ['java'] : [])]
  static languages = process.env.TAC_WINDOW_LANGUAGES?.split(',') ?? WindowControlsGate.allLanguages
  static grants = ['minimize', 'resize', 'title']
  static notGranted = 'The close window control is not granted. Add "close" to tac.application.window.controls in client/manifest.json.'
  static noWindow = 'The minimize window control needs the native app window.'
  static browserTimeout = 30_000

  static sources = {
    js: ['client/services/JsWindowService.js', `@Service
export default class JsWindowService {
  /** @type {TacWindowControls} */
  @Needs
  windowControls;

  /** @returns {boolean} */
  allowed() {
    return this.windowControls.granted('minimize');
  }

  /** @returns {string} */
  shrink() {
    return this.#refusal(() => this.windowControls.minimize());
  }

  /** @returns {string} */
  shut() {
    return this.#refusal(() => this.windowControls.close());
  }

  /** @returns {string} */
  grow() {
    return this.#refusal(() => this.windowControls.resize(1280, 800));
  }

  /** @returns {string} */
  rename() {
    return this.#refusal(() => this.windowControls.setTitle('Report'));
  }

  /**
   * @param {() => void} call
   * @returns {string}
   */
  #refusal(call) {
    try {
      call();
      return '';
    } catch (error) {
      return error.message;
    }
  }
}
`],
    ts: ['client/services/TsWindowService.ts', `@Service
export default class TsWindowService {
  @Needs
  windowControls!: TacWindowControls;

  allowed(): boolean {
    return this.windowControls.granted('minimize');
  }

  shrink(): string {
    return this.refusal(() => this.windowControls.minimize());
  }

  shut(): string {
    return this.refusal(() => this.windowControls.close());
  }

  grow(): string {
    return this.refusal(() => this.windowControls.resize(1280, 800));
  }

  rename(): string {
    return this.refusal(() => this.windowControls.setTitle('Report'));
  }

  private refusal(call: () => void): string {
    try {
      call();
      return '';
    } catch (error) {
      return (error as Error).message;
    }
  }
}
`],
    rust: ['client/services/RustWindowService.rs', `#[Service]
#[derive(Default)]
struct RustWindowService {
    window_controls: WindowControls,
}

impl RustWindowService {
    pub fn allowed(&self) -> bool {
        self.window_controls.granted("minimize").unwrap_or(false)
    }

    pub fn shrink(&self) -> String {
        self.window_controls.minimize().err().unwrap_or_default()
    }

    pub fn shut(&self) -> String {
        self.window_controls.close().err().unwrap_or_default()
    }

    pub fn grow(&self) -> String {
        self.window_controls.resize(1280, 800).err().unwrap_or_default()
    }

    pub fn rename(&self) -> String {
        self.window_controls.set_title("Report").err().unwrap_or_default()
    }
}
`],
    go: ['client/services/GoWindowService.go', `// @Service
type GoWindowService struct {
	// @Needs
	windowControls WindowControls
}

func (service *GoWindowService) Allowed() bool {
	granted, _ := service.windowControls.Granted("minimize")
	return granted
}

func (service *GoWindowService) Shrink() string {
	return service.refusal(service.windowControls.Minimize())
}

func (service *GoWindowService) Shut() string {
	return service.refusal(service.windowControls.Close())
}

func (service *GoWindowService) Grow() string {
	return service.refusal(service.windowControls.Resize(1280, 800))
}

func (service *GoWindowService) Rename() string {
	return service.refusal(service.windowControls.SetTitle("Report"))
}

func (service *GoWindowService) refusal(problem error) string {
	if problem == nil {
		return ""
	}
	return problem.Error()
}
`],
    csharp: ['client/services/CsharpWindowService.cs', `[Service]
public class CsharpWindowService
{
    private readonly WindowControls windowControls = new();

    public bool Allowed() => windowControls.Granted("minimize");

    public string Shrink() => Refusal(() => windowControls.Minimize());

    public string Shut() => Refusal(() => windowControls.Close());

    public string Grow() => Refusal(() => windowControls.Resize(1280, 800));

    public string Rename() => Refusal(() => windowControls.SetTitle("Report"));

    private string Refusal(System.Action call)
    {
        try
        {
            call();
            return "";
        }
        catch (System.InvalidOperationException error)
        {
            return error.Message;
        }
    }
}
`],
    fsharp: ['client/services/FsharpWindowService.fs', `[<Service>]
type FsharpWindowService() =
    let windowControls = WindowControls()

    member this.Allowed() : bool = windowControls.Granted("minimize")

    member this.Shrink() : string = this.Refusal(fun () -> windowControls.Minimize())

    member this.Shut() : string = this.Refusal(fun () -> windowControls.Close())

    member this.Grow() : string = this.Refusal(fun () -> windowControls.Resize(1280, 800))

    member this.Rename() : string = this.Refusal(fun () -> windowControls.SetTitle("Report"))

    member private this.Refusal(call: unit -> unit) : string =
        try
            call ()
            ""
        with :? System.InvalidOperationException as error -> error.Message
`],
    kotlin: ['client/services/KotlinWindowService.kt', `@Service
class KotlinWindowService {
    private val windowControls = WindowControls()

    fun allowed(): Boolean = windowControls.granted("minimize")

    fun shrink(): String = refusal { windowControls.minimize() }

    fun shut(): String = refusal { windowControls.close() }

    fun grow(): String = refusal { windowControls.resize(1280, 800) }

    fun rename(): String = refusal { windowControls.setTitle("Report") }

    private fun refusal(call: () -> Unit): String =
        try {
            call()
            ""
        } catch (error: IllegalStateException) {
            error.message.orEmpty()
        }
}
`],
    dart: ['client/services/DartWindowService.dart', `@Service()
class DartWindowService {
  @Needs()
  final WindowControls windowControls;

  bool allowed() => windowControls.granted('minimize');

  String shrink() => _refusal(() => windowControls.minimize());

  String shut() => _refusal(() => windowControls.close());

  String grow() => _refusal(() => windowControls.resize(1280, 800));

  String rename() => _refusal(() => windowControls.setTitle('Report'));

  String _refusal(void Function() call) {
    try {
      call();
      return '';
    } on StateError catch (error) {
      return error.message;
    }
  }
}
`],
    ruby: ['client/services/RubyWindowService.rb', `#[Service]
class RubyWindowService
  def allowed
    WindowControls.new.granted("minimize")
  end

  def shrink
    refusal { WindowControls.new.minimize }
  end

  def shut
    refusal { WindowControls.new.close }
  end

  def grow
    refusal { WindowControls.new.resize(1280, 800) }
  end

  def rename
    refusal { WindowControls.new.set_title("Report") }
  end

  private

  def refusal
    yield
    ""
  rescue RuntimeError => error
    error.message
  end
end
`],
    swift: ['client/services/SwiftWindowService.swift', `@Service
final class SwiftWindowService {
    private let windowControls = WindowControls()

    func allowed() throws -> Bool {
        return try windowControls.granted("minimize")
    }

    func shrink() -> String {
        refusal { try windowControls.minimize() }
    }

    func shut() -> String {
        refusal { try windowControls.close() }
    }

    func grow() -> String {
        refusal { try windowControls.resize(width: 1280, height: 800) }
    }

    func rename() -> String {
        refusal { try windowControls.setTitle("Report") }
    }

    private func refusal(_ call: () throws -> Void) -> String {
        do {
            try call()
            return ""
        } catch {
            return String(describing: error)
        }
    }
}
`],
    java: ['client/services/JavaWindowService.java', `@Service
final class JavaWindowService {
    private final WindowControls windowControls = new WindowControls();

    boolean allowed() {
        return windowControls.granted("minimize");
    }

    String shrink() {
        return refusal(windowControls::minimize);
    }

    String shut() {
        return refusal(windowControls::close);
    }

    String grow() {
        return refusal(() -> windowControls.resize(1280, 800));
    }

    String rename() {
        return refusal(() -> windowControls.setTitle("Report"));
    }

    private String refusal(Runnable call) {
        try {
            call.run();
            return "";
        } catch (IllegalStateException refused) {
            return refused.getMessage();
        }
    }
}
`],
  }

  static files() {
    const files = {
      'client/manifest.json': '{"tac":{"names":{"words":["js","ts","csharp","fsharp"]},"visibility":{"check":false}}}\n',
      'client/pages/tac.html': '<window-page />\n',
      'client/components/window/page/tac.html': `<main>${WindowControlsGate.languages.map((language) => `<button id="${language}" on:click="${language}Window.shrink()">Minimise</button>`).join('\n')}</main>\n`,
    }
    for (const language of WindowControlsGate.languages) {
      const [file, source] = WindowControlsGate.sources[language]
      files[file] = source
    }
    return files
  }

  static write(project) {
    for (const [relative, contents] of Object.entries(WindowControlsGate.files())) {
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
      const built = spawnSync(WindowControlsGate.binary, ['build', project], {
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
    const built = spawnSync(WindowControlsGate.binary, ['build', project], {
      encoding: 'utf8', timeout: 900_000, killSignal: 'SIGKILL', maxBuffer: 8 * 1024 * 1024,
    })
    if (built.status !== 0) throw new Error(`window controls fixture build failed: ${built.error?.message || built.stderr || built.stdout}`)
  }

  static handler(project) {
    const root = path.join(project, 'dist')
    return (request, response) => {
      const url = new URL(request.url, 'http://localhost')
      let file = path.join(root, decodeURIComponent(url.pathname))
      if (existsSync(file) && statSync(file).isDirectory()) file = path.join(file, 'index.html')
      if (!existsSync(file)) {
        response.writeHead(404).end()
        return
      }
      response.writeHead(200, { 'content-type': WindowControlsGate.mimeTypes[path.extname(file)] ?? 'application/octet-stream' })
      createReadStream(file).pipe(response)
    }
  }

  static expect(actual, wanted, label) {
    if (actual !== wanted) throw new Error(`${label}: expected ${JSON.stringify(wanted)}, got ${JSON.stringify(actual)}`)
    console.log(`ok - ${label}`)
  }

  static ask(page, language, call) {
    return page.evaluate(([name, method]) => __tachyonTac.service(`${name}Window`)[method](), [language, call])
  }

  static nativeHost = (grants) => {
    globalThis.__tachyonHostCalls = []
    globalThis.__tachyonNativeHostCall = async (capability, payload) => {
      if (capability.startsWith('window.')) globalThis.__tachyonHostCalls.push([capability, JSON.parse(payload)])
      return JSON.stringify({ value: { fields: [], methods: [] } })
    }
    globalThis.__tachyonWindowControls = Object.freeze(grants)
  }

  // A component resolves its services one after another, so a service can register after the network is idle.
  static registered(page) {
    const names = WindowControlsGate.languages.map((language) => `${language}Window`)
    return page.waitForFunction((wanted) => wanted.every((name) => globalThis.__tachyonTac?.service(name)), names, { timeout: WindowControlsGate.browserTimeout })
  }

  static async checkTab(page, browser, language) {
    const label = `${browser} ${language}`
    WindowControlsGate.expect(await WindowControlsGate.ask(page, language, 'allowed'), false, `${label}: a browser tab grants no window control`)
    const refusal = await WindowControlsGate.ask(page, language, 'shrink')
    WindowControlsGate.expect(refusal.includes(WindowControlsGate.noWindow), true, `${label}: a window call in a browser tab fails and says why (${refusal})`)
  }

  static async checkNative(page, browser, language) {
    const label = `${browser} ${language}`
    await page.evaluate(() => { globalThis.__tachyonHostCalls.length = 0 })
    WindowControlsGate.expect(await WindowControlsGate.ask(page, language, 'allowed'), true, `${label}: granted says a granted control is granted`)
    for (const call of ['shrink', 'grow', 'rename']) {
      WindowControlsGate.expect(await WindowControlsGate.ask(page, language, call), '', `${label}: ${call} reaches the native host`)
    }
    const refusal = await WindowControlsGate.ask(page, language, 'shut')
    WindowControlsGate.expect(refusal.includes(WindowControlsGate.notGranted), true, `${label}: a control that is not granted fails and names the grant (${refusal})`)
    const sent = JSON.stringify(await page.evaluate(() => globalThis.__tachyonHostCalls))
    WindowControlsGate.expect(sent, JSON.stringify([
      ['window.minimize', {}],
      ['window.resize', { width: 1280, height: 800 }],
      ['window.title', { title: 'Report' }],
    ]), `${label}: the host receives only the granted calls`)
  }

  static async run(origin, name) {
    const browser = await WindowControlsGate.launchers[name].launch({ timeout: WindowControlsGate.browserTimeout })
    try {
      const tab = await browser.newPage()
      await tab.goto(`${origin}/`, { waitUntil: 'networkidle' })
      await WindowControlsGate.registered(tab)
      for (const language of WindowControlsGate.languages) await WindowControlsGate.checkTab(tab, name, language)
      const context = await browser.newContext()
      await context.addInitScript(WindowControlsGate.nativeHost, WindowControlsGate.grants)
      const native = await context.newPage()
      await native.goto(`${origin}/`, { waitUntil: 'networkidle' })
      await WindowControlsGate.registered(native)
      for (const language of WindowControlsGate.languages) await WindowControlsGate.checkNative(native, name, language)
    } finally {
      await browser.close()
    }
  }

  static async main() {
    const project = mkdtempSync(path.join(tmpdir(), 'tachyon-window-controls-'))
    const server = createServer(WindowControlsGate.handler(project))
    try {
      WindowControlsGate.write(project)
      WindowControlsGate.checkBudgetRefusal()
      WindowControlsGate.build(project)
      await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve))
      const origin = `http://127.0.0.1:${server.address().port}`
      for (const name of WindowControlsGate.browsers) await WindowControlsGate.run(origin, name)
    } finally {
      server.close()
      rmSync(project, { recursive: true, force: true })
    }
  }
}

await WindowControlsGate.main()
