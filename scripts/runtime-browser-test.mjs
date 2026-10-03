#!/usr/bin/env node
import { CargoArtifacts } from './cargo-artifacts.mjs'
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createReadStream, existsSync, mkdirSync, mkdtempSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

class RuntimeGate {
  static repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('release/ty');
  static iconSource = '/shared/icon" data-manifest-probe="&marker.png';
  static iconSizes = '32x32"><script type="module" src="/shared/manifest-injection.js"></script><link sizes="32x32';
  static strictPolicy = "default-src 'self'; script-src 'self'; style-src 'self'; connect-src 'self'";
  static webAssemblyPolicy = "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self'; connect-src 'self'";
  static precacheLimit = 4 * 1024 * 1024;
}

class RuntimeModels {
  static home = [['Counter', 'integer', 'value'], ['Method', 'string', 'text'], ['Returned', 'integer', 'value']]
  static native = [['Early', 'string', 'text']]
  static go = [['GoCounted', 'integer', 'count']]
  static mesh = [
    ['OrderPlaced', 'integer', 'total'],
    ['OrderReturned', 'string', 'language'],
    ['OrderRetained', 'integer', 'total'],
    ['NativeOrdered', 'string', 'language'],
    ['NativeFailure', 'string', 'language'],
    ['LateHeard', 'string', 'language'],
  ]
  static rust = [
    ['OrderPlaced', 'pub struct OrderPlaced {\n    pub total: i64,\n}\n'],
    ['OrderReturned', 'pub struct OrderReturned {\n    pub language: String,\n}\n'],
    ['OrderSettled', 'pub struct OrderSettled {\n    pub total: i64,\n}\n'],
    ['OrderRetained', 'pub struct OrderRetained {\n    pub total: i64,\n}\n'],
    ['NativeOrdered', 'pub struct NativeOrdered {\n    pub language: String,\n}\n'],
    ['NativeFailure', 'pub struct NativeFailure {\n    pub language: String,\n}\n'],
    ['LateHeard', 'pub struct LateHeard {\n    pub language: String,\n}\n'],
  ]
  static goStructs = [
    ['OrderPlaced', 'type OrderPlaced struct {\n\tTotal int `json:"total"`\n}\n'],
    ['OrderReturned', 'type OrderReturned struct {\n\tLanguage string `json:"language"`\n}\n'],
    ['OrderSettled', 'type OrderSettled struct {\n\tTotal int `json:"total"`\n}\n'],
    ['OrderRetained', 'type OrderRetained struct {\n\tTotal int `json:"total"`\n}\n'],
    ['NativeOrdered', 'type NativeOrdered struct {\n\tLanguage string `json:"language"`\n}\n'],
    ['NativeFailure', 'type NativeFailure struct {\n\tLanguage string `json:"language"`\n}\n'],
    ['LateHeard', 'type LateHeard struct {\n\tLanguage string `json:"language"`\n}\n'],
  ]
  static csharp = [
    ['OrderPlaced', 'public record OrderPlaced(int Total);\n'],
    ['OrderReturned', 'public record OrderReturned(string Language);\n'],
    ['OrderRetained', 'public record OrderRetained(int Total);\n'],
    ['NativeOrdered', 'public record NativeOrdered(string Language);\n'],
    ['NativeFailure', 'public record NativeFailure(string Language);\n'],
    ['LateHeard', 'public record LateHeard(string Language);\n'],
  ]
}

function write(project, name, source) {
  const file = path.join(project, name);
  mkdirSync(path.dirname(file), { recursive: true });
  writeFileSync(file, source);
}

function writeJavaScriptModels(project, models) {
  for (const [name, type, field] of models) {
    write(project, `client/models/${name}.js`, `/**\n * @typedef {Object} ${name}\n * @property {${type}} ${field}\n */\nexport {}\n`);
  }
}

async function closeServer(server) {
  if (!server) return;
  const closed = new Promise((resolve, reject) => server.close((error) => {
    if (error && error.code !== 'ERR_SERVER_NOT_RUNNING') reject(error);
    else resolve();
  }));
  server.closeAllConnections();
  await closed;
}

function writeHomeFixtures(project) {
  write(project, 'client/manifest.json', JSON.stringify({ icons: [{ src: RuntimeGate.iconSource, sizes: RuntimeGate.iconSizes }], tac: { visibility: { check: false } } }));
  writeJavaScriptModels(project, RuntimeModels.home);
  write(project, 'client/shared/manifest-injection.js', 'globalThis.manifestInjectionExecuted = true;');
  write(project, 'client/pages/tac.html', '<runtime-home />');
  write(project, 'client/components/runtime/home/tac.html', `<main><p id="count">{runtimeHome.count}</p><p id="seen">{runtimeHome.seen}</p>
<button id="increment" on:click="runtimeHome.increment()">Increment</button>
<button id="hide" on:click="runtimeHome.visible = false">Hide</button>
<logic :if="runtimeHome.visible"><signal-panel /></logic></main>`);
  write(project, 'client/services/RuntimeHomeService.js', `@Service()
export default class RuntimeHomeService {
  applicationEventPublisher;
  @SessionState
  count = 2;
  seen = 0;
  visible = true;
  @OnStart()
  initialize() { this.counter(this.count); }
  /** @param {Counter} counter */
  @OnEvent({"replay":true})
  receive(counter) { this.seen = counter.value; }
  counter(value) {
    /** @type {Counter} */
    const counter = { value };
    this.applicationEventPublisher.publishEvent(counter, true);
  }
  method(text) {
    /** @type {Method} */
    const method = { text };
    this.applicationEventPublisher.publishEvent(method);
  }
  increment() { this.count += 1; this.counter(this.count); }
  async answer() {
    /** @type {Returned} */
    const returned = { value: 42 };
    this.applicationEventPublisher.publishEvent(returned);
    return 42;
  }
  async reject() { throw new Error('private text'); }
}`);
  write(project, 'client/components/signal/panel/tac.html', '<p id="panel">{signalPanel.seen}</p>');
  write(project, 'client/services/SignalPanelService.js', `@Service()
@Lifetime({"value":"prototype"})
export default class SignalPanelService {
  seen = 0;
  /** @param {Counter} counter */
  @OnEvent({"replay":true})
  counter(counter) { this.seen = counter.value; }
  /** @param {Method} method */
  @OnEvent
  receiveMethod(method) { document.documentElement.dataset.delivery = method.text; }
  /** @param {Returned} returned */
  @OnEvent
  receiveReturned(returned) { document.documentElement.dataset.delivery = String(returned.value); }
}`);
}

function writeNativeFixtures(project) {
  write(project, 'client/pages/native/tac.html', '<runtime-native />');
  write(project, 'client/components/runtime/native/tac.html', '<main><p id="native-value">{runtimeNative.value}</p><input aria-label="Native name" :value="runtimeNative.value" on:input="runtimeNative.value = $event.target.value"><button id="native-append" on:click="runtimeNative.value += &quot;!&quot;">Append</button><button id="native-argument" on:click="runtimeNative.act(7)">Call</button><input id="native-explicit-event" aria-label="Explicit event" on:input="runtimeNative.act($event.target.value)"></main>');
  write(project, 'client/services/RuntimeNativeService.rs', '#[Service]\n#[derive(Default)]\nstruct RuntimeNativeService { value: String, early: String }\nimpl RuntimeNativeService {\n    fn act(&self, value: String) -> String { value }\n}\n');
  writeJavaScriptModels(project, RuntimeModels.native);
  write(project, 'client/services/RuntimeNativeService.js', `@Service()
export default class RuntimeNativeService {
  value = 'browser';
  early = null;
  /** @param {Early} early */
  @OnEvent
  receive(early) { this.early = early.text; }
  act() { throw new Error('native transport was not selected'); }
}`);
}

function writeLoopAndMountFixtures(project) {
  write(project, 'client/pages/loops/tac.html', '<runtime-loops />');
  write(project, 'client/components/runtime/loops/tac.html', '﻿' + `<main>
<section id="ascending"><loop :for="let i = runtimeLoops.start; i < runtimeLoops.limit; i += runtimeLoops.step"><span>{i}</span></loop></section>
<section id="descending"><loop :for="let i = 4; i >= 0; i -= 2"><span>{i}</span></loop></section>
<section id="inclusive"><loop :for="let i = 0; i <= 2; i++"><span>{i}</span></loop></section>
<section id="nested"><loop :for="group of runtimeLoops.groups"><loop :for="let i = 0; i < group.length; i++"><span>{group.length}:{i}:{$index}</span></loop></loop></section>
<section id="empty-nested"><loop :for="let i = 0; i < runtimeLoops.emptyLimit; i++"><loop :for="let j = 0; j < runtimeLoops.emptyLimit; j++"></loop></loop></section>
<p id="entities" title="&amp;lt;">A&nbsp;&amp;&lt;tag&gt;&amp;amp;</p>
</main>`);
  write(project, 'client/services/RuntimeLoopsService.js', "@Service()\nexport default class RuntimeLoopsService { start = 0; limit = 3; step = 1; emptyLimit = 0; groups = [[1, 2], [3]]; }");
  write(project, 'client/pages/mounts/tac.html', '<runtime-mounts />');
  write(project, 'client/components/runtime/mounts/tac.html', '<main><logic :if="runtimeMounts.show"><delayed-idle hydrate="idle"/><delayed-click hydrate="interaction"/><div id="spacer"></div><delayed-visible hydrate="visible"/></logic></main>');
  write(project, 'client/components/runtime/mounts/tac.css', '#spacer { height: 5000px; }');
  write(project, 'client/services/RuntimeMountsService.js', "@Service()\nexport default class RuntimeMountsService { show = true; }");
  write(project, 'client/pages/focus-boundaries/tac.html', '<runtime-boundaries />');
  write(project, 'client/components/runtime/boundaries/tac.html', `<main><p id="boundary-status">{runtimeBoundaries.status}</p>
<section id="rows"><loop :for="row of runtimeBoundaries.rows"><input aria-label="Row" :value="row.label" on:input="runtimeBoundaries.edit(row.id, $event.target.value)"></loop></section>
<unknown-wrap><input aria-label="Wrapped" :value="runtimeBoundaries.text" on:input="runtimeBoundaries.text = $event.target.value"></unknown-wrap>
<observed-wrap :mode="runtimeBoundaries.mode"><input aria-label="Observed" :value="runtimeBoundaries.text" on:input="runtimeBoundaries.text = $event.target.value"></observed-wrap>
</main>`);
  write(project, 'client/services/RuntimeBoundariesService.js', `@Service()
export default class RuntimeBoundariesService {
  rows = [{ id: 1, label: 'First' }]; text = 'abcdef'; mode = 'before'; status = 'before';
  edit(_event, id, value) { this.#rowWithId(id).label = value; }
  #rowWithId(id) { return this.rows.find((row) => row.id === id); }
}`);
  for (const name of ['idle', 'click', 'visible']) {
    const service = `Delayed${name[0].toUpperCase()}${name.slice(1)}`;
    write(project, `client/components/delayed/${name}/tac.html`, `<button>{${service[0].toLowerCase()}${service.slice(1)}.label}</button>`);
    write(project, `client/services/${service}Service.js`, `  @Service()
  export default class ${service}Service {
    label = '${name}';
    @OnStart()
    initialize() { document.delayedActivations.push('${name}'); }
  }`);
  }
}

function writeFocusFixtures(project) {
  write(project, 'client/pages/focus/tac.html', '<runtime-focus />');
  write(project, 'client/components/runtime/focus/tac.html', `<main><h1>Focus regression</h1>
<label>Customer name<input aria-label="Customer name" :value="runtimeFocus.customer" on:input="runtimeFocus.customer = $event.target.value"></label>
<output id="customer">{runtimeFocus.customer}</output>
<input id="identified" aria-label="Identified input" :value="runtimeFocus.identified" on:input="runtimeFocus.identified = $event.target.value">
<textarea aria-label="Notes" on:input="runtimeFocus.notes = $event.target.value">{runtimeFocus.notes}</textarea>
<logic :if="runtimeFocus.visible"><focus-shell><focus-editor /></focus-shell></logic>
<button on:click="runtimeFocus.visible = false">Remove editor</button></main>`);
  write(project, 'client/services/RuntimeFocusService.js', "@Service()\nexport default class RuntimeFocusService { customer = ''; identified = ''; notes = ''; visible = true; }");
  write(project, 'client/components/focus/editor/tac.html', '<section><label>Component input<input aria-label="Component input" :value="focusEditor.value" on:input="focusEditor.value = $event.target.value"></label><output id="component-value">{focusEditor.value}</output></section>');
  write(project, 'client/services/FocusEditorService.js', "@Service()\nexport default class FocusEditorService { value = ''; }");
}

function writeGoFixtures(project) {
  write(project, 'client/pages/tac.html', '<go-counter />');
  write(project, 'client/components/go/counter/tac.html', `<main><p id="go-count">{goCounter.count}</p><p id="go-label">{goCounter.label}</p>
<p id="go-seen">{goCounter.seen}</p><p id="go-watched">{goWatcher.last}</p>
<button id="go-increment" on:click="goCounter.increment()">Increment</button>
<button id="go-add" on:click="goCounter.add(5)">Add five</button>
<input aria-label="Go label" :value="goCounter.label" on:input="goCounter.label = $event.target.value"></main>`);
  write(project, 'client/services/GoCounterService.go', `// @Service()
type GoCounterService struct {
\tCount     int
\tLabel     string
\tSeen      int
\tpublisher ApplicationEventPublisher
}

// @OnStart()
func (counter *GoCounterService) Start() {
\tcounter.Label = "ready"
}

func (counter *GoCounterService) Increment() int {
\tcounter.Count++
\tcounter.publisher.PublishEvent(GoCounted{Count: counter.Count}, false)
\treturn counter.Count
}

func (counter *GoCounterService) Add(by int) int {
\tcounter.Count += by
\treturn counter.Count
}

// @OnEvent
func (counter *GoCounterService) Counted(value GoCounted) {
\tcounter.Seen = value.Count
}
`);
  write(project, 'client/models/GoCounted.go', 'type GoCounted struct {\n\tCount int `json:"count"`\n}\n');
  writeJavaScriptModels(project, RuntimeModels.go);
  write(project, 'client/services/GoWatcherService.js', `@Service()
export default class GoWatcherService {
  last = 0;
  /** @param {GoCounted} counted */
  @OnEvent
  receive(counted) { this.last = counted.count; }
}`);
}

function goAvailable() {
  const probe = spawnSync('go', ['version'], { encoding: 'utf8' });
  return !probe.error && probe.status === 0;
}

async function checkGoService(browser) {
  if (!goAvailable()) {
    assert.equal(process.env.CI, undefined, 'CI must provide Go for the Go web service check');
    console.log('skipping the Go web service check: go is not installed');
    return;
  }
  const project = mkdtempSync(path.join(tmpdir(), 'ty-runtime-go-'));
  let server;
  try {
    writeGoFixtures(project);
    buildProject(project);
    const dist = path.join(project, 'dist');
    const wasm = statSync(path.join(dist, '.tachyon/services/go/services.wasm')).size;
    assert.ok(wasm < RuntimeGate.precacheLimit, `the Go module (${wasm} bytes) stays under the service worker precache limit`);
    server = serveDist(dist, RuntimeGate.webAssemblyPolicy);
    await listen(server);
    const errors = [];
    const page = await browser.newPage();
    await page.addInitScript(rootHelper);
    await page.addInitScript(definePageFileSystem);
    page.setDefaultTimeout(15000);
    page.on('pageerror', (error) => errors.push(error.message));
    await page.goto(`http://127.0.0.1:${server.address().port}/`);
    await page.waitForFunction(() => document.querySelector('#go-label')?.textContent === 'ready');
    assert.equal(await page.evaluate(() => fs(3)), 3, 'a page script keeps its own global fs beside the Go module');
    assert.equal(await page.textContent('#go-count'), '0', 'the view reads a Go field');
    await page.click('#go-increment');
    await page.waitForFunction(() => document.querySelector('#go-count')?.textContent === '1');
    await page.waitForFunction(() => document.querySelector('#go-watched')?.textContent === '1');
    assert.equal(await page.textContent('#go-seen'), '1', 'a Go listener receives its own publication');
    await page.click('#go-add');
    await page.waitForFunction(() => document.querySelector('#go-count')?.textContent === '6');
    await page.getByRole('textbox', { name: 'Go label', exact: true }).fill('typed');
    await page.waitForFunction(() => document.querySelector('#go-label')?.textContent === 'typed');
    assert.equal(await page.evaluate(() => __root('goCounter').increment()), 7, 'a direct call returns the Go value');
    assert.deepEqual(errors, [], 'the Go page runs without errors');
    console.log(`Go web service module: ${wasm} bytes`);
  } finally {
    await closeServer(server);
    rmSync(project, { recursive: true, force: true });
  }
}

function writeRubyFixtures(project) {
  write(project, 'client/pages/tac.html', '<ruby-layout :open="false" />');
  write(project, 'client/components/ruby/layout/tac.html', '<ruby-drawer :open="open" />');
  write(project, 'client/components/ruby/drawer/tac.html', `<main>
<button id="ruby-toggle" on:click="rubyDrawer.toggle()" :aria-expanded="rubyDrawer.open ? 'true' : 'false'">Menu</button>
<nav id="ruby-panel" :data-open="rubyDrawer.open ? 'true' : 'false'" on:keydown="rubyDrawer.dismiss($event.key)"><a id="ruby-link" href="#top">Top</a></nav>
<p id="ruby-count">{rubyCounter.count}</p>
<button id="ruby-increment" on:click="rubyCounter.increment()">Increment</button></main>`);
  write(project, 'client/services/RubyDrawerService.rb', `#[Service]
#[NeedsAll]
class RubyDrawerService
  attr_reader :open

  def toggle
    @open = !@open
  end

  def dismiss(key)
    close_on_escape(key)
  end

  private

  def close
    @open = false
  end

  def close_on_escape(key)
    close if key == "Escape"
  end
end
`);
  write(project, 'client/services/RubyCounterService.rb', `#[Service]
class RubyCounterService
  attr_reader :count

  def initialize
    @count = 0
  end

  def increment
    @count += 1
  end
end
`);
}

async function checkRubyService(browser) {
  const project = mkdtempSync(path.join(tmpdir(), 'ty-runtime-ruby-'));
  let server;
  try {
    writeRubyFixtures(project);
    buildProject(project);
    server = serveDist(path.join(project, 'dist'), RuntimeGate.webAssemblyPolicy);
    await listen(server);
    const errors = [];
    const page = await browser.newPage();
    page.setDefaultTimeout(15000);
    page.on('pageerror', (error) => errors.push(error.message));
    await page.goto(`http://127.0.0.1:${server.address().port}/`);
    await page.waitForFunction(() => document.querySelector('#ruby-count')?.textContent === '0');
    const settled = async () => {
      await page.evaluate(() => __tachyonTac.render());
      return page.evaluate(() => [
        document.querySelector('#ruby-toggle').getAttribute('aria-expanded'),
        document.querySelector('#ruby-panel').dataset.open,
        document.querySelector('#ruby-count').textContent,
      ]);
    };
    assert.deepEqual(await settled(), ['false', 'false', '0'], 'the Ruby drawer starts from the prop its parents pass');
    await page.click('#ruby-increment');
    assert.deepEqual(await settled(), ['false', 'false', '1'], 'a Ruby method call shows its new field at once');
    await page.click('#ruby-increment');
    assert.deepEqual(await settled(), ['false', 'false', '2'], 'every Ruby method call shows its own new field');
    await page.click('#ruby-toggle');
    assert.deepEqual(await settled(), ['true', 'true', '2'], 'the Ruby drawer opens on the first click');
    await page.click('#ruby-toggle');
    assert.deepEqual(await settled(), ['false', 'false', '2'], 'the Ruby drawer closes on the second click');
    await page.click('#ruby-toggle');
    await page.focus('#ruby-link');
    await page.keyboard.press('Escape');
    assert.deepEqual(await settled(), ['false', 'false', '2'], 'Escape closes the Ruby drawer at once');
    assert.deepEqual(errors, [], 'the Ruby page runs without errors');
  } finally {
    await closeServer(server);
    rmSync(project, { recursive: true, force: true });
  }
}

function writeLuaFixtures(project) {
  write(project, 'client/pages/tac.html', '<lua-layout :open="false" />');
  write(project, 'client/components/lua/layout/tac.html', '<lua-drawer :open="open" />');
  write(project, 'client/components/lua/drawer/tac.html', `<main>
<button id="lua-toggle" on:click="luaDrawer.toggle()" :aria-expanded="luaDrawer.open ? 'true' : 'false'">Menu</button>
<nav id="lua-panel" :data-open="luaDrawer.open ? 'true' : 'false'" on:keydown="luaDrawer.dismiss($event.key)"><a id="lua-link" href="#top">Top</a></nav>
<p id="lua-count">{luaCounter.count}</p>
<p id="lua-notes">{luaCounter.notes.length}</p>
<button id="lua-increment" on:click="luaCounter.increment()">Increment</button></main>`);
  write(project, 'client/services/LuaDrawerService.lua', `-- @Service
-- @NeedsAll
---@class LuaDrawerService
LuaDrawerService = {
  ---@type boolean
  open = nil,
}

---@return boolean
function LuaDrawerService:toggle()
  self.open = not self.open
  return self.open
end

---@param key string
---@return nil
function LuaDrawerService:dismiss(key)
  self:_close_on_escape(key)
end

---@param key string
---@return nil
function LuaDrawerService:_close_on_escape(key)
  if key == "Escape" then
    self.open = false
  end
end
`);
  write(project, 'client/services/LuaCounterService.lua', `-- @Service
---@class LuaCounterService
LuaCounterService = {
  ---@type integer
  count = 0,
  ---@type string[]
  notes = {},
}

---@return integer
function LuaCounterService:increment()
  self.count = self.count + 1
  self.notes[#self.notes + 1] = "clicked"
  return self.count
end
`);
}

async function checkLuaService(browser) {
  const project = mkdtempSync(path.join(tmpdir(), 'ty-runtime-lua-'));
  let server;
  try {
    writeLuaFixtures(project);
    buildProject(project);
    server = serveDist(path.join(project, 'dist'), RuntimeGate.webAssemblyPolicy);
    await listen(server);
    const errors = [];
    const page = await browser.newPage();
    page.setDefaultTimeout(15000);
    page.on('pageerror', (error) => errors.push(error.message));
    await page.goto(`http://127.0.0.1:${server.address().port}/`);
    await page.waitForFunction(() => document.querySelector('#lua-count')?.textContent === '0');
    const settled = async () => {
      await page.evaluate(() => __tachyonTac.render());
      return page.evaluate(() => [
        document.querySelector('#lua-toggle').getAttribute('aria-expanded'),
        document.querySelector('#lua-panel').dataset.open,
        document.querySelector('#lua-count').textContent,
        document.querySelector('#lua-notes').textContent,
      ]);
    };
    assert.deepEqual(await settled(), ['false', 'false', '0', '0'], 'the Lua drawer starts from the prop its parents pass');
    await page.click('#lua-increment');
    assert.deepEqual(await settled(), ['false', 'false', '1', '1'], 'a Lua method call shows its new fields at once');
    await page.click('#lua-increment');
    assert.deepEqual(await settled(), ['false', 'false', '2', '2'], 'every Lua method call shows its own new fields');
    await page.click('#lua-toggle');
    assert.deepEqual(await settled(), ['true', 'true', '2', '2'], 'the Lua drawer opens on the first click');
    await page.click('#lua-toggle');
    assert.deepEqual(await settled(), ['false', 'false', '2', '2'], 'the Lua drawer closes on the second click');
    await page.click('#lua-toggle');
    await page.focus('#lua-link');
    await page.keyboard.press('Escape');
    assert.deepEqual(await settled(), ['false', 'false', '2', '2'], 'Escape closes the Lua drawer at once');
    assert.deepEqual(errors, [], 'the Lua page runs without errors');
  } finally {
    await closeServer(server);
    rmSync(project, { recursive: true, force: true });
  }
}

function writeEventMeshFixtures(project, withCSharp) {
  write(project, 'client/pages/tac.html', '<event-mesh />');
  write(project, 'client/components/event/mesh/tac.html', `<main>
<button id="send-large" on:click="signal.send(25)">Large</button><button id="send-small" on:click="signal.send(5)">Small</button>
<p id="rust-records">{rustAudit.records}</p><p id="rust-settled">{rustAudit.settled}</p>
<p id="go-records">{goAudit.records}</p><p id="go-large">{goAudit.large}</p><p id="go-heard">{goAudit.heard}</p>
<p id="first-saw">{signal.firstSaw}</p><p id="last-saw">{signal.lastSaw}</p>
<p id="returned">{signal.returned}</p>
<button id="retain" on:click="signal.retain()">Retain</button>
<button id="show-late" on:click="signal.showLate = true">Show late</button>
<button id="hide-late" on:click="signal.showLate = false">Hide late</button>
<button id="send-native-go" on:click="signal.sendNative('go')">Go event</button>
<button id="send-native-rust" on:click="signal.sendNative('rust')">Rust event</button>
${withCSharp ? '<button id="send-native-cs" on:click="signal.sendNative(\'cs\')">C# event</button>' : ''}
<button id="send-failure-go" on:click="signal.sendFailure('go')">Go failure</button>
<button id="send-failure-rust" on:click="signal.sendFailure('rust')">Rust failure</button>
${withCSharp ? '<button id="send-failure-cs" on:click="signal.sendFailure(\'cs\')">C# failure</button>' : ''}
<p id="healthy-deliveries">{signal.healthyDeliveries}</p><p id="failed-publications">{signal.failedPublications}</p>
<p id="late-deliveries">{signal.lateDeliveries}</p>
<p id="native-before">{signal.nativeBefore}</p><p id="native-after">{signal.nativeAfter}</p>
<p id="native-async-during">{signal.nativeAsyncDuring}</p><p id="native-async">{goAudit.ownAsync}</p>
<logic :if="signal.showLate"><event-late /></logic>
${withCSharp ? '<p id="cs-records">{csAudit.records}</p>' : ''}</main>`);
  writeJavaScriptModels(project, RuntimeModels.mesh);
  write(project, 'client/services/SignalService.js', `@Service()
export default class SignalService {
  static _AUDIT = 'Audit';
  static _SEPARATOR = ',';
  applicationEventPublisher;
  firstSaw = -1;
  lastSaw = -1;
  returned = '';
  showLate = false;
  lateDeliveries = 0;
  nativeBefore = -1;
  nativeAfter = -1;
  nativeAsyncDuring = -1;
  healthyDeliveries = 0;
  failedPublications = 0;
  sendFailure(_event, language) {
    /** @type {NativeFailure} */
    const failure = { language };
    this.applicationEventPublisher.publishEvent(failure);
    this.failedPublications += 1;
  }
  /** @param {NativeFailure} _failure */
  @OnEvent
  @Order(200)
  healthy(_failure) { this.healthyDeliveries += 1; }
  retain() {
    /** @type {OrderRetained} */
    const retained = { total: 17 };
    this.applicationEventPublisher.publishEvent(retained, true);
  }
  send(_event, total) {
    /** @type {OrderPlaced} */
    const placed = { total };
    this.applicationEventPublisher.publishEvent(placed);
  }
  sendNative(_event, language) {
    const audit = __tachyonTac.service(language + SignalService._AUDIT);
    audit.emitOrdered();
    this.nativeAsyncDuring = audit.ownAsync;
  }
  /** @param {NativeOrdered} ordered */
  @OnEvent
  @Order(0)
  nativeFirst(ordered) { this.nativeBefore = __tachyonTac.service(ordered.language + SignalService._AUDIT).ownOrdered; }
  /** @param {NativeOrdered} ordered */
  @OnEvent
  @Order(200)
  nativeLast(ordered) { this.nativeAfter = __tachyonTac.service(ordered.language + SignalService._AUDIT).ownOrdered; }
  _records() {
    return this.#recordsOf(['rustAudit', 'goAudit'${withCSharp ? ", 'csAudit'" : ''}]);
  }
  #recordsOf(services) {
    return services.map((name) => __tachyonTac.service(name).records).join(SignalService._SEPARATOR);
  }
  /** @param {OrderPlaced} _placed */
  @OnEvent
  @Order(1)
  before(_placed) { this.firstSaw = this._records(); }
  /** @param {OrderPlaced} _placed */
  @OnEvent
  @Order(3)
  after(_placed) { this.lastSaw = this._records(); }
  /** @param {OrderReturned} returned */
  @OnEvent
  receive(returned) { this.returned += returned.language + SignalService._SEPARATOR; }
  /** @param {LateHeard} _heard */
  @OnEvent
  heard(_heard) { this.lateDeliveries += 1; }
}`);
  for (const [name, source] of RuntimeModels.rust) write(project, `client/models/${name}.rs`, source);
  for (const [name, source] of RuntimeModels.goStructs) write(project, `client/models/${name}.go`, source);
  write(project, 'client/services/RustAuditService.rs', `use crate::models::{NativeFailure, NativeOrdered, OrderPlaced, OrderReturned, OrderSettled};

#[Service]
#[derive(Default)]
struct RustAuditService {
    records: i64,
    settled: i64,
    ownOrdered: i64,
    ownAsync: i64,
    application_event_publisher: ApplicationEventPublisher,
}

impl RustAuditService {
    const LANGUAGE: &str = "rust";

    #[OnEvent({"condition":"value.total > 10"})]
    #[Order(2)]
    fn record(&mut self, _value: OrderPlaced) -> OrderReturned {
        self.records += 1;
        OrderReturned { language: String::from(Self::LANGUAGE) }
    }

    #[OnEvent]
    fn save_settled(&mut self, value: OrderSettled) { self.settled = value.total; }

    fn emitOrdered(&self) {
        self.application_event_publisher.publish_event(NativeOrdered { language: String::from(Self::LANGUAGE) }, false);
    }

    #[OnEvent({"condition":"value.language == 'rust'"})]
    #[Order(100)]
    fn ordered(&mut self, _value: NativeOrdered) { self.ownOrdered += 1; }

    #[OnEvent({"condition":"value.language == 'rust'"})]
    #[InBackground]
    fn deferred(&mut self, _value: NativeOrdered) { self.ownAsync += 1; }

    #[OnEvent({"condition":"value.language == 'rust'"})]
    fn fail(&self, _value: NativeFailure) -> Result<(), String> {
        Err(String::from("private-rust-listener-failure"))
    }
}
`);
  write(project, 'client/services/GoAuditService.go', `import "errors"

// @Service()
type GoAuditService struct {
\tRecords   int
\tLarge     int
\tHeard     int
\tOwnOrdered int
\tOwnAsync int
\tpublisher ApplicationEventPublisher
}

// @OnEvent({"condition":"value.total > 10"})
// @Order(2)
func (audit *GoAuditService) Record(value OrderPlaced) OrderReturned {
\taudit.Records++
\treturn OrderReturned{Language: audit.language()}
}

// @OnEvent({"condition":"value.total > 10"})
// @Order(2)
func (audit *GoAuditService) Settle(value OrderPlaced) {
\taudit.Large++
\taudit.publisher.PublishEvent(OrderSettled{Total: value.Total}, false)
}

// @OnEvent
func (audit *GoAuditService) Hear(value OrderSettled) {
\taudit.Heard++
}

func (audit *GoAuditService) EmitOrdered() {
\taudit.publisher.PublishEvent(NativeOrdered{Language: audit.language()}, false)
}

// @OnEvent({"condition":"value.language == 'go'"})
// @Order(100)
func (audit *GoAuditService) Ordered(value NativeOrdered) {
\taudit.OwnOrdered++
}

// @OnEvent({"condition":"value.language == 'go'"})
// @InBackground
func (audit *GoAuditService) Deferred(value NativeOrdered) {
\taudit.OwnAsync++
}

// @OnEvent({"condition":"value.language == 'go'"})
func (audit *GoAuditService) Fail(value NativeFailure) error {
\treturn errors.New("private-go-listener-failure")
}

func (GoAuditService) language() string { return "go" }
`);
  writeLateEventFixtures(project, withCSharp);
  if (!withCSharp) return;
  for (const [name, source] of RuntimeModels.csharp) write(project, `client/models/${name}.cs`, source);
  write(project, 'client/services/CsAuditService.cs', `[Service]
public class CsAuditService
{
    private const string Language = "cs";
    public int Records = 0;
    public int OwnOrdered = 0;
    public int OwnAsync = 0;
    private ApplicationEventPublisher publisher = new();

    public void EmitOrdered() { publisher.PublishEvent(new Models.NativeOrdered(Language), false); }

    [OnEvent({"condition":"value.language == 'cs'"})]
    [Order(100)]
    public void Ordered(Models.NativeOrdered value) { OwnOrdered += 1; }

    [OnEvent({"condition":"value.language == 'cs'"})]
    [InBackground]
    public void Deferred(Models.NativeOrdered value) { OwnAsync += 1; }

    [OnEvent({"condition":"value.language == 'cs'"})]
    public void Fail(Models.NativeFailure value) { throw new Exception("private-cs-listener-failure"); }

    [OnEvent({"condition":"value.total > 10"})]
    [Order(2)]
    public Models.OrderReturned Record(Models.OrderPlaced value)
    {
        Records += 1;
        return new Models.OrderReturned(Language);
    }
}
`);
}

function writeLateEventFixtures(project, withCSharp) {
  write(project, 'client/components/event/late/tac.html', `<section id="late-listeners">
<p id="rust-replay">{rustLate.value}</p><p id="go-replay">{goLate.value}</p>
${withCSharp ? '<p id="cs-replay">{csLate.value}</p>' : ''}</section>`);
  write(project, 'client/services/RustLateService.rs', `use crate::models::{LateHeard, OrderRetained};

#[Service]
#[Lifetime({"value":"prototype"})]
#[derive(Default)]
struct RustLateService { value: i64 }
impl RustLateService {
    #[OnEvent({"replay":true})]
    fn receive(&mut self, value: OrderRetained) -> LateHeard {
        self.value = value.total;
        LateHeard { language: String::from("rust") }
    }
}
`);
  write(project, 'client/services/GoLateService.go', `// @Service()
// @Lifetime({"value":"prototype"})
type GoLateService struct { Value int }

// @OnEvent({"replay":true})
func (late *GoLateService) Receive(value OrderRetained) LateHeard {
\tlate.Value = value.Total
\treturn LateHeard{Language: "go"}
}
`);
  if (!withCSharp) return;
  write(project, 'client/services/CsLateService.cs', `[Service]
[Lifetime({"value":"prototype"})]
public class CsLateService
{
    public int Value = 0;
    [OnEvent({"replay":true})]
    public Models.LateHeard Receive(Models.OrderRetained value)
    {
        Value = value.Total;
        return new Models.LateHeard("cs");
    }
}
`);
}

function csharpWebAvailable() {
  const probe = spawnSync('dotnet', ['workload', 'list'], {
    encoding: 'utf8', timeout: 30000, maxBuffer: 1024 * 1024,
  });
  return !probe.error && probe.status === 0 && probe.stdout.includes('wasm-tools');
}

async function checkWebAssemblyEvents(browser) {
  if (!goAvailable()) {
    assert.equal(process.env.CI, undefined, 'CI must provide Go for the WebAssembly event check');
    console.log('skipping the WebAssembly event check: go is not installed');
    return;
  }
  const withCSharp = csharpWebAvailable();
  if (!withCSharp) {
    assert.equal(process.env.CI, undefined, 'CI must provide .NET wasm-tools for the C# WebAssembly event check');
    console.log('the WebAssembly event check runs without C#: the .NET wasm-tools workload is not installed');
  }
  const project = mkdtempSync(path.join(tmpdir(), 'ty-runtime-wasm-events-'));
  let server;
  try {
    writeEventMeshFixtures(project, withCSharp);
    buildProject(project);
    server = serveDist(path.join(project, 'dist'), RuntimeGate.webAssemblyPolicy);
    await listen(server);
    const errors = [];
    const page = await browser.newPage();
    page.setDefaultTimeout(30000);
    page.on('pageerror', (error) => errors.push(error.message));
    await page.goto(`http://127.0.0.1:${server.address().port}/`);
    await page.waitForFunction(() => document.querySelector('#rust-records')?.textContent === '0' && document.querySelector('#go-records')?.textContent === '0');
    const text = async (id) => page.textContent(`#${id}`);
    await page.click('#send-large');
    await page.waitForFunction(() => document.querySelector('#rust-settled')?.textContent === '25');
    assert.equal(await text('rust-records'), '1', 'a Rust listener hears a JavaScript publication');
    assert.equal(await text('go-records'), '1', 'a Go listener hears a JavaScript publication');
    assert.equal(await text('go-large'), '1', 'a Go listener whose condition holds runs');
    assert.equal(await text('go-heard'), '1', 'a Go listener hears its own publication once, not again from the page');
    const languages = withCSharp ? ['cs', 'go', 'rust'] : ['go', 'rust'];
    assert.equal(await text('first-saw'), languages.map(() => 0).join(','), 'lower Order runs before every WASM listener');
    assert.equal(await text('last-saw'), languages.map(() => 1).join(','), 'higher Order runs after every WASM listener');
    assert.deepEqual((await text('returned')).split(',').filter(Boolean).sort(), languages,
      'each WASM listener republishes its returned event exactly once to JavaScript');
    if (withCSharp) assert.equal(await text('cs-records'), '1', 'a C# listener hears a JavaScript publication');
    await page.click('#send-small');
    await page.waitForFunction((count) => document.querySelector('#first-saw')?.textContent === Array(count).fill(1).join(','), languages.length);
    assert.equal(await text('rust-records'), '1', 'a Rust listener whose condition fails does not run');
    assert.equal(await text('go-records'), '1', 'a Go listener whose condition fails does not run');
    assert.equal(await text('go-large'), '1', 'a Go listener whose condition fails does not run');
    assert.equal(await text('rust-settled'), '25', 'no settlement follows a small order');
    assert.equal(await text('go-heard'), '1', 'the Go listener heard only the one settlement');
    if (withCSharp) assert.equal(await text('cs-records'), '1', 'a C# listener whose condition fails does not run');
    assert.deepEqual((await text('returned')).split(',').filter(Boolean).sort(), languages,
      'filtered listeners do not publish return values');
    await checkLateWebAssemblyListeners(page, languages);
    await checkWebAssemblyOriginOrder(page, languages);
    assert.equal(await page.evaluate(() => document.documentElement.dataset.tachyonSignalError), undefined,
      'the event graph reports no swallowed listener failures');
    await checkWebAssemblyListenerFailures(page, languages);
    assert.deepEqual(errors, [], 'the event page runs without errors');
    console.log(`WebAssembly event delivery passed: ${languages.join(', ')}; typed models across languages, conditions, Order, returned models, replay and disposal`);
  } finally {
    await closeServer(server);
    rmSync(project, { recursive: true, force: true });
  }
}

async function checkWebAssemblyListenerFailures(page, languages) {
  for (const [index, language] of languages.entries()) {
    await page.evaluate(() => { delete document.documentElement.dataset.tachyonSignalError; });
    await page.click(`#send-failure-${language}`);
    const result = await page.evaluate(() => {
      const signal = __tachyonTac.service('signal');
      return {
        error: document.documentElement.dataset.tachyonSignalError,
        healthy: signal.healthyDeliveries,
        completed: signal.failedPublications,
      };
    });
    assert.deepEqual(result, { error: 'listener_failed', healthy: index + 1, completed: index + 1 },
      `${language}: listener failure is observable without interrupting the publisher or healthy peers`);
  }
}

async function checkWebAssemblyOriginOrder(page, languages) {
  for (const language of languages) {
    await page.click(`#send-native-${language}`);
    await page.waitForFunction((name) => __tachyonTac.service(name + 'Audit').ownAsync === 1, language);
    assert.equal(await page.textContent('#native-before'), '0', `${language}: JavaScript Order(0) precedes the publisher's Order(100)`);
    assert.equal(await page.textContent('#native-after'), '1', `${language}: JavaScript Order(200) follows the publisher's Order(100)`);
    assert.equal(await page.textContent('#native-async-during'), '0', `${language}: the publisher's Async listener waits until the browser microtask`);
  }
}

async function checkLateWebAssemblyListeners(page, languages) {
  await page.click('#retain');
  await page.click('#show-late');
  for (const language of languages) {
    await page.waitForFunction((name) => document.querySelector(`#${name}-replay`)?.textContent === '17', language);
  }
  const deliveries = () => page.evaluate(() => __tachyonTac.service('signal').lateDeliveries);
  assert.equal(await deliveries(), languages.length, 'late WASM listeners replay the retained event exactly once');
  await page.click('#hide-late');
  await page.waitForFunction(() => !document.querySelector('#late-listeners'));
  await page.click('#retain');
  assert.equal(await deliveries(), languages.length, 'disposed WASM listeners receive no further events');
  await page.click('#show-late');
  await page.waitForFunction((count) => __tachyonTac.service('signal').lateDeliveries === count * 2, languages.length);
  assert.equal(await deliveries(), languages.length * 2, 'replacement WASM owners register one fresh replay listener each');
}

const rootHelper = () => { globalThis.__root = (name) => __tachyonTac.service(name); };

function definePageFileSystem() {
  globalThis.fs = (value) => value;
}

function buildProject(project, command = ['build', project]) {
  const build = spawnSync(RuntimeGate.binary, command, {
    encoding: 'utf8', timeout: 120_000, killSignal: 'SIGKILL', maxBuffer: 1024 * 1024,
  });
  assert.ifError(build.error);
  assert.equal(build.status, 0, build.stderr || build.stdout);
}

function serveDist(output, policy = RuntimeGate.strictPolicy) {
  return createServer((request, response) => {
    const pathname = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
    let file = path.resolve(output, `.${pathname}`);
    if (!file.startsWith(`${output}${path.sep}`) && file !== output) { response.writeHead(403).end(); return; }
    if (existsSync(file) && statSync(file).isDirectory()) file = path.join(file, 'index.html');
    if (!existsSync(file)) { response.writeHead(404).end(); return; }
    response.setHeader('content-type', contentType(file));
    response.setHeader('content-security-policy', policy);
    createReadStream(file).pipe(response);
  });
}

function contentType(file) {
  const types = { '.js': 'text/javascript', '.css': 'text/css', '.wasm': 'application/wasm' };
  return types[path.extname(file)] ?? 'text/html';
}

function listen(server) {
  return new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', () => { server.off('error', reject); resolve(); });
  });
}

async function checkHomePage(browser, origin, errors) {
  const page = await browser.newPage();
  await page.addInitScript(rootHelper);
  page.setDefaultTimeout(10000);
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto(origin);
  await page.waitForSelector('#panel');
  const manifestHead = await page.evaluate(() => ({
    executed: globalThis.manifestInjectionExecuted === true,
    injectedScripts: document.querySelectorAll('script[src="/shared/manifest-injection.js"]').length,
    injectedAttributes: document.querySelectorAll('[data-manifest-probe]').length,
    icons: document.querySelectorAll('link[rel="icon"]').length,
    href: document.querySelector('link[rel="icon"]')?.getAttribute('href'),
    sizes: document.querySelector('link[rel="icon"]')?.getAttribute('sizes'),
    appleHref: document.querySelector('link[rel="apple-touch-icon"]')?.getAttribute('href'),
  }));
  assert.deepEqual(manifestHead, { executed: false, injectedScripts: 0, injectedAttributes: 0,
    icons: 1, href: RuntimeGate.iconSource, sizes: RuntimeGate.iconSizes, appleHref: RuntimeGate.iconSource },
  'manifest values stay literal attributes and cannot inject executable same-origin markup');
  assert.equal(await page.textContent('#seen'), '2', 'explicit page subscriber receives retained default');
  assert.equal(await page.textContent('#panel'), '2', 'prototype service subscribes during mount');
  await page.click('#increment');
  await page.waitForFunction(() => document.querySelector('#panel')?.textContent === '3');
  await page.reload();
  await page.waitForFunction(() => document.querySelector('#seen')?.textContent === '3');
  assert.equal(await page.textContent('#count'), '3', 'explicit publication composes with persistent fields');
  const result = await page.evaluate(async () => {
    const instance = __root('runtimeHome');
    const answer = await instance.answer();
    try { await instance.reject(); } catch {}
    return { answer, hidden: !('tac' in instance) && !('tac' in __tachyonTac),
      cloned: structuredClone({ ...instance }).count };
  });
  assert.deepEqual(result, { answer: 42, hidden: true, cloned: 3 });
  assert.equal(await page.getAttribute('html', 'data-delivery'), '42', 'each model has its own typed listener');
  await page.evaluate(() => __root('runtimeHome').method('before'));
  assert.equal(await page.getAttribute('html', 'data-delivery'), 'before');
  await page.evaluate(() => __tachyonTac.hotUpdate(['service:SignalPanelService'], 'runtime-test'));
  await page.evaluate(() => __root('runtimeHome').method('after-hot'));
  assert.equal(await page.getAttribute('html', 'data-delivery'), 'after-hot');
  await page.click('#hide');
  await page.waitForFunction(() => !document.querySelector('#panel'));
  await page.evaluate(() => __root('runtimeHome').method('removed'));
  assert.equal(await page.getAttribute('html', 'data-delivery'), 'after-hot', 'removed prototype service disposed with its component');
  assert.deepEqual(errors, [], 'no uncaught runtime errors or CSP violations');
}

async function checkFocus(browser, origin, errors) {
  const focus = await browser.newPage();
  await focus.addInitScript(rootHelper);
  await focus.addInitScript(() => {
    customElements.define('focus-shell', class extends HTMLElement {
      connectedCallback() {
        if (this.querySelector('slot')) return;
        const slot = document.createElement('slot');
        slot.append(...this.childNodes);
        const wrapper = document.createElement('header');
        wrapper.append(slot);
        this.append(wrapper);
      }
    });
  });
  focus.setDefaultTimeout(10000);
  focus.on('pageerror', (error) => errors.push(error.message));
  await focus.goto(`${origin}/focus/`);
  const customer = focus.getByRole('textbox', { name: 'Customer name', exact: true });
  await customer.click();
  await focus.evaluate(() => { globalThis.originalInput = document.activeElement; });
  await customer.pressSequentially('Customer', { delay: 35 });
  assert.equal(await customer.inputValue(), 'Customer', 'typing must not drop characters from an input without an ID');
  await focus.waitForFunction(() => document.querySelector('#customer')?.textContent === 'Customer');
  assert.equal(await focus.evaluate(() => document.activeElement === originalInput && originalInput.isConnected), true,
    'reactive updates retain the focused DOM node, not only its ID');
  await customer.press('Home');
  await customer.press('ArrowRight');
  await customer.pressSequentially('X', { delay: 35 });
  assert.equal(await customer.inputValue(), 'CXustomer', 'middle insertion preserves caret');
  await focus.waitForFunction(() => document.querySelector('#customer')?.textContent === 'CXustomer');
  assert.equal(await customer.evaluate((input) => input.selectionStart), 2);
  const identified = focus.getByRole('textbox', { name: 'Identified input', exact: true });
  await identified.fill('abcdef');
  await identified.evaluate((input) => input.setSelectionRange(1, 4, 'backward'));
  await focus.evaluate(() => __tachyonTac.render());
  assert.deepEqual(await identified.evaluate((input) => [input.selectionStart, input.selectionEnd, input.selectionDirection]), [1, 4, 'backward']);
  const notes = focus.getByRole('textbox', { name: 'Notes', exact: true });
  await notes.click();
  await notes.pressSequentially('line one', { delay: 35 });
  await notes.press('Enter');
  await notes.pressSequentially('line two', { delay: 35 });
  assert.equal(await notes.inputValue(), 'line one\nline two', 'textarea typing and line breaks retain focus');
  const componentInput = focus.getByRole('textbox', { name: 'Component input', exact: true });
  await componentInput.click();
  await focus.evaluate(() => { globalThis.originalComponentInput = document.activeElement; });
  await componentInput.pressSequentially('component', { delay: 35 });
  await focus.waitForFunction(() => document.querySelector('#component-value')?.textContent === 'component');
  assert.equal(await focus.evaluate(() => document.activeElement === originalComponentInput && originalComponentInput.isConnected), true,
    'focused descendants remain connected through component host updates');
  await componentInput.dispatchEvent('compositionstart', { data: '' });
  await focus.evaluate(() => __tachyonTac.render());
  assert.equal(await focus.evaluate(() => document.activeElement === originalComponentInput), true, 'composition does not lose the composing node');
  await componentInput.dispatchEvent('compositionend', { data: '' });
  await focus.evaluate(async () => { __root('runtimeFocus').visible = false; await __tachyonTac.render(); });
  assert.equal(await componentInput.count(), 0, 'removing a focused control still removes its component');
  assert.equal(await focus.evaluate(() => originalComponentInput.isConnected), false);
  assert.deepEqual(errors, [], 'focus updates do not introduce uncaught errors');
}

async function checkFocusBoundaries(browser, origin) {
  const boundaries = await browser.newPage();
  await boundaries.addInitScript(rootHelper);
  await boundaries.addInitScript(() => {
    globalThis.observedLiveMutations = 0;
    customElements.define('unknown-wrap', class extends HTMLElement {
      connectedCallback() {
        if (this.querySelector('[data-vendor]')) return;
        const wrapper = document.createElement('div');
        wrapper.dataset.vendor = 'unknown';
        wrapper.append(...this.childNodes);
        this.append(wrapper);
      }
    });
    customElements.define('observed-wrap', class extends HTMLElement {
      static observedAttributes = ['mode'];
      attributeChangedCallback() {
        if (!this.isConnected) return;
        observedLiveMutations += 1;
        this.replaceChildren(document.createTextNode('vendor replaced editing subtree'));
      }
    });
  });
  await boundaries.goto(`${origin}/focus-boundaries/`);
  const row = boundaries.getByRole('textbox', { name: 'Row', exact: true });
  await row.click();
  const replacedRow = await boundaries.evaluate(async () => {
    const old = document.activeElement;
    __root('runtimeBoundaries').rows = [{ id: 2, label: 'Replacement' }];
    await __tachyonTac.render();
    const current = document.querySelector('#rows input');
    return { oldConnected: old.isConnected, focusTransferred: document.activeElement === current, value: current.value };
  });
  assert.deepEqual(replacedRow, { oldConnected: false, focusTransferred: false, value: 'Replacement' },
    'replacing an unkeyed lexical row must not transfer its editing focus to another record');
  const wrapped = boundaries.getByRole('textbox', { name: 'Wrapped', exact: true });
  await wrapped.click();
  await wrapped.evaluate((input) => input.setSelectionRange(1, 3, 'backward'));
  const wrapperFallback = await boundaries.evaluate(async () => {
    const old = document.activeElement;
    __root('runtimeBoundaries').status = 'after-wrapper';
    await __tachyonTac.render();
    const current = document.querySelector('[aria-label="Wrapped"]');
    return { replaced: old !== current && !old.isConnected, focused: document.activeElement === current,
      selection: [current.selectionStart, current.selectionEnd, current.selectionDirection],
      status: document.querySelector('#boundary-status').textContent };
  });
  assert.deepEqual(wrapperFallback, { replaced: true, focused: true, selection: [1, 3, 'backward'], status: 'after-wrapper' },
    'an unknown wrapper falls back to a complete update without a partial commit');
  const observed = boundaries.getByRole('textbox', { name: 'Observed', exact: true });
  await observed.click();
  await observed.evaluate((input) => input.setSelectionRange(2, 2));
  const attributeFallback = await boundaries.evaluate(async () => {
    const old = document.activeElement;
    __root('runtimeBoundaries').mode = 'after';
    __root('runtimeBoundaries').status = 'after-observed';
    await __tachyonTac.render();
    const current = document.querySelector('[aria-label="Observed"]');
    return { replaced: old !== current && !old.isConnected, focused: document.activeElement === current,
      caret: current.selectionStart, liveMutations: observedLiveMutations,
      mode: document.querySelector('observed-wrap').getAttribute('mode'),
      status: document.querySelector('#boundary-status').textContent };
  });
  assert.deepEqual(attributeFallback, { replaced: true, focused: true, caret: 2, liveMutations: 0, mode: 'after', status: 'after-observed' },
    'observed-attribute fallback is decided before touching the connected custom element');
}

async function checkMountCancellation(browser, origin) {
  const mounts = await browser.newPage();
  await mounts.addInitScript(rootHelper);
  await mounts.addInitScript(() => {
    document.delayedActivations = [];
    globalThis.cancelledIdle = [];
    globalThis.queuedIdle = new Map();
    globalThis.idleSequence = 0;
    globalThis.requestIdleCallback = (callback) => { const id = ++idleSequence; queuedIdle.set(id, callback); return id; };
    globalThis.cancelIdleCallback = (id) => { cancelledIdle.push(id); queuedIdle.delete(id); };
    const Observer = globalThis.IntersectionObserver;
    globalThis.disconnectedObservers = 0;
    globalThis.IntersectionObserver = class extends Observer {
      disconnect() { disconnectedObservers += 1; super.disconnect(); }
    };
  });
  await mounts.goto(`${origin}/mounts/`);
  await mounts.waitForSelector('delayed-click, tachyon-component[data-tachyon-component="delayed-click"]');
  const removedMounts = await mounts.evaluate(async () => {
    const before = [...queuedIdle.values()];
    const interaction = document.querySelector('[data-tachyon-component="delayed-click"] button');
    __root('runtimeMounts').show = false;
    await __tachyonTac.render();
    for (const callback of before) await callback({ didTimeout: false, timeRemaining: () => 50 });
    interaction.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, cancelable: true }));
    await Promise.resolve();
    return { activated: document.delayedActivations, idleCancelled: cancelledIdle.length > 0,
      visibleDisconnected: disconnectedObservers > 0, hosts: document.querySelectorAll('tachyon-component[data-tachyon-component^="delayed-"]').length };
  });
  assert.deepEqual(removedMounts, { activated: [], idleCancelled: true, visibleDisconnected: true, hosts: 0 },
    'removed component schedules are cancelled and stale activation is rejected');
}

async function checkLoops(browser, origin) {
  const loops = await browser.newPage();
  await loops.addInitScript(rootHelper);
  await loops.goto(`${origin}/loops/`);
  await loops.waitForSelector('#entities');
  const values = async (selector) => loops.locator(`${selector} span`).allTextContents();
  assert.deepEqual(await values('#ascending'), ['0', '1', '2']);
  assert.deepEqual(await values('#descending'), ['4', '2', '0']);
  assert.deepEqual(await values('#inclusive'), ['0', '1', '2']);
  assert.deepEqual(await values('#nested'), ['2:0:0', '2:1:1', '1:0:0']);
  assert.equal(await loops.textContent('#entities'), 'A\u00a0&<tag>&amp;', 'BOM stripped and text entities decoded once');
  assert.equal(await loops.getAttribute('#entities', 'title'), '&lt;', 'attribute entities decoded once');
  await loops.evaluate(async () => { __root('runtimeLoops').limit = 5; await __tachyonTac.render(); });
  assert.deepEqual(await values('#ascending'), ['0', '1', '2', '3', '4'], 'counted bound rerenders');
  for (const step of [0, -1, Infinity, NaN]) {
    await loops.evaluate(async (value) => { __root('runtimeLoops').step = value; await __tachyonTac.render(); }, step);
    assert.deepEqual(await values('#ascending'), [], `invalid dynamic step ${step} terminates`);
  }
  const bounded = await loops.evaluate(async () => {
    __root('runtimeLoops').step = 1;
    __root('runtimeLoops').limit = 10001;
    let timer;
    try {
      return await Promise.race([
        __tachyonTac.render().then(() => 'unexpected-success', (error) => error instanceof RangeError ? 'bounded-error' : 'wrong-error'),
        new Promise((resolve) => { timer = setTimeout(() => resolve('timeout'), 5000); }),
      ]);
    } finally { clearTimeout(timer); }
  });
  assert.equal(bounded, 'bounded-error', 'iteration cap fails rather than hanging the renderer');
  assert.equal(await loops.getAttribute('html', 'data-tachyon-render-error'), 'render_failed');
  await loops.evaluate(async () => { __root('runtimeLoops').limit = 3; await __tachyonTac.render(); });
  assert.deepEqual(await values('#ascending'), ['0', '1', '2'], 'renderer recovers after bounded failure');
  assert.equal(await loops.getAttribute('html', 'data-tachyon-render-error'), null);

  const nestedBounded = await loops.evaluate(async () => {
    __root('runtimeLoops').emptyLimit = 10000;
    let timer;
    try {
      return await Promise.race([
        __tachyonTac.render().then(() => 'unexpected-success', (error) => error instanceof RangeError ? 'bounded-error' : 'wrong-error'),
        new Promise((resolve) => { timer = setTimeout(() => resolve('timeout'), 5000); }),
      ]);
    } finally { clearTimeout(timer); }
  });
  assert.equal(nestedBounded, 'bounded-error', 'aggregate iteration budget bounds nested empty loops as well as emitted nodes');
  await loops.evaluate(async () => { __root('runtimeLoops').emptyLimit = 0; await __tachyonTac.render(); });
}

async function checkNativeBridge(browser, origin) {
  const native = await browser.newPage();
  await native.addInitScript(rootHelper);
  await native.addInitScript(() => {
    globalThis.nativeCalls = [];
    let stored = 'native-ready';
    globalThis.__tachyonCompanionQueue = [{ name: 'Early', value: { text: 'queued' } }];
    globalThis.__tachyonNativeHostCall = async (_capability, payload) => {
      const request = JSON.parse(payload);
      nativeCalls.push(request);
      if (request.op === 'init') globalThis.__tachyonCompanionQueue.push({ route: request.route, service: request.service, owner: request.owner, name: 'Early', value: { text: 'queued' } });
      if (request.op === 'set') stored = request.value;
      const value = request.op === 'init' ? { fields: ['value', '__proto__'], methods: ['act'] }
        : request.op === 'get' ? stored : request.args?.[0];
      return JSON.stringify({ value });
    };
  });
  await native.goto(`${origin}/native/`);
  await native.waitForSelector('#native-value');
  assert.equal(await native.textContent('#native-value'), 'native-ready');
  const bridge = await native.evaluate(async () => {
    const atStart = nativeCalls.filter((call) => call.op === 'call').length;
    const answer = await __root('runtimeNative').act('called');
    __root('runtimeNative').value = 'changed';
    await __tachyonTac.render();
    globalThis.__tachyonCompanionPublish({ route: '/different', service: 'runtimeNative', owner: nativeCalls[0].owner, name: 'Early', value: { text: 'forged-route' } });
    globalThis.__tachyonCompanionPublish({ route: '/native', service: 'runtimeNative', owner: 'old-owner', name: 'Early', value: { text: 'stale-owner' } });
    globalThis.__tachyonCompanionPublish({ route: '/native', service: 'otherService', owner: nativeCalls[0].owner, name: 'Early', value: { text: 'forged-service' } });
    return { atStart, answer, queued: __root('runtimeNative').early, owned: nativeCalls.every((call) => typeof call.owner === 'string' && call.owner === nativeCalls[0].owner),
      routes: nativeCalls.every((call) => call.route === '/native'),
      services: nativeCalls.every((call) => call.service === 'runtimeNative'),
      reserved: nativeCalls.some((call) => call.name === '__proto__'),
      set: nativeCalls.some((call) => call.op === 'set' && call.value === 'changed'),
      rendered: document.querySelector('#native-value').textContent };
  });
  assert.deepEqual(bridge, { atStart: 0, answer: 'called', queued: 'queued', owned: true, routes: true, services: true, reserved: false, set: true, rendered: 'changed' });
  const tooBig = await native.evaluate(async () => {
    try { await __root('runtimeNative').act('x'.repeat(70000)); return 'accepted'; } catch (error) { return error.message; }
  });
  assert.equal(tooBig, 'Native service runtimeNative: The request to act is 70129 bytes. The limit is 65536 bytes. Pass less data to the service.');
  await native.click('#native-argument');
  await native.waitForFunction(() => nativeCalls.some((call) => call.op === 'call' && call.args?.[0] === 7));
  await native.fill('#native-explicit-event', 'authored');
  await native.waitForFunction(() => nativeCalls.some((call) => call.op === 'call' && call.args?.[0] === 'authored'));
  assert.deepEqual(await native.evaluate(() => nativeCalls.filter((call) => call.op === 'call').map((call) => call.args)),
    [['called'], [7], ['authored']], 'native events pass only authored arguments; direct calls are unchanged');
}

async function checkDelayedNativeEdits(browser, origin) {
  const delayedNative = await browser.newPage();
  await delayedNative.addInitScript(rootHelper);
  await delayedNative.addInitScript(() => {
    let stored = '';
    globalThis.delayedHost = {
      hold: false, pending: [],
      releaseOne() { this.pending.shift()?.resolve(); },
      releaseAll() { this.hold = false; while (this.pending.length) this.releaseOne(); },
    };
    globalThis.__tachyonNativeHostCall = async (_capability, payload) => {
      const request = JSON.parse(payload);
      if (request.op === 'set') stored = request.value;
      const value = request.op === 'init' ? { fields: ['value'], methods: ['act'] }
        : request.op === 'get' ? stored : request.args?.[0];
      const reply = JSON.stringify({ value });
      if (request.op === 'get' && delayedHost.hold) {
        return new Promise(resolve => delayedHost.pending.push({ value, resolve: () => resolve(reply) }));
      }
      return reply;
    };
  });
  await delayedNative.goto(`${origin}/native/`);
  const nativeName = delayedNative.getByRole('textbox', { name: 'Native name', exact: true });
  await nativeName.click();
  await delayedNative.evaluate(() => { globalThis.originalNativeInput = document.activeElement; delayedHost.hold = true; });
  await nativeName.pressSequentially('A');
  await delayedNative.waitForFunction(() => delayedHost.pending[0]?.value === 'A');
  await nativeName.pressSequentially('d');
  await delayedNative.evaluate(() => delayedHost.releaseOne());
  await delayedNative.waitForFunction(() => delayedHost.pending[0]?.value === 'Ad');
  await delayedNative.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  assert.equal(await nativeName.inputValue(), 'Ad', 'an older native acknowledgement cannot erase a newer edit');
  assert.equal(await delayedNative.evaluate(() => document.activeElement === originalNativeInput && originalNativeInput.isConnected), true);
  await nativeName.pressSequentially('a');
  await delayedNative.evaluate(() => delayedHost.releaseAll());
  await delayedNative.waitForFunction(() => document.querySelector('#native-value')?.textContent === 'Ada');
  assert.equal(await nativeName.inputValue(), 'Ada');

  await delayedNative.evaluate(() => { delayedHost.hold = true; void __root('runtimeNative').act('refresh'); });
  await delayedNative.waitForFunction(() => delayedHost.pending[0]?.value === 'Ada');
  await nativeName.pressSequentially('X');
  await delayedNative.waitForFunction(() => delayedHost.pending.length === 2);
  await delayedNative.evaluate(() => delayedHost.releaseOne());
  await delayedNative.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  assert.equal(await nativeName.inputValue(), 'AdaX', 'method refresh cannot overwrite an in-flight native edit');
  await delayedNative.click('#native-append');
  await delayedNative.evaluate(() => delayedHost.releaseAll());
  await delayedNative.waitForFunction(() => document.querySelector('#native-value')?.textContent === 'AdaX!');
  assert.equal(await nativeName.inputValue(), 'AdaX!', 'a subsequent user action sees the newest native field edit');
}

async function main() {
  let project;
  let server;
  let nativeServer;
  let browser;
  try {
    project = mkdtempSync(path.join(tmpdir(), 'ty-runtime-gate-'));
    writeHomeFixtures(project);
    writeNativeFixtures(project);
    writeLoopAndMountFixtures(project);
    writeFocusFixtures(project);
    buildProject(project);
    buildProject(project, ['bundle', project, '--native', '--target', 'linux']);
    server = serveDist(path.join(project, 'dist'));
    await listen(server);
    const origin = `http://127.0.0.1:${server.address().port}`;
    nativeServer = serveDist(path.join(project, 'dist/linux/web'));
    await listen(nativeServer);
    const nativeOrigin = `http://127.0.0.1:${nativeServer.address().port}`;
    browser = await chromium.launch({ timeout: 30_000 });
    const errors = [];
    await checkHomePage(browser, origin, errors);
    await checkFocus(browser, origin, errors);
    await checkFocusBoundaries(browser, origin);
    await checkMountCancellation(browser, origin);
    await checkLoops(browser, origin);
    await checkNativeBridge(browser, nativeOrigin);
    await checkDelayedNativeEdits(browser, nativeOrigin);
    await checkGoService(browser);
    await checkRubyService(browser);
    await checkLuaService(browser);
    await checkWebAssemblyEvents(browser);
    console.log('Tac runtime browser gate passed: service lifecycle, persistence, explicit signals, HMR disposal, counted loops/bounds, BOM/entities, native transport, strict CSP, Go, Ruby and Lua WebAssembly services, WebAssembly event delivery.');
  } finally {
    try {
      const cleanup = await Promise.allSettled([browser?.close(), closeServer(server), closeServer(nativeServer)]);
      for (const result of cleanup) {
        if (result.status === 'rejected') { console.error('runtime cleanup failed:', result.reason); process.exitCode = 1; }
      }
    } finally {
      if (project) rmSync(project, { recursive: true, force: true });
    }
  }
}

await main();
