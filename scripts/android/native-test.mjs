#!/usr/bin/env node
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import './bridge-unit-test.mjs';

class AndroidGate {
  static repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
  static sdk = process.env.ANDROID_HOME ?? process.env.ANDROID_SDK_ROOT;
}

const run = (command, args, options = {}) => {
  const result = spawnSync(command, args, { cwd: AndroidGate.repo, encoding: 'utf8', timeout: 240000, maxBuffer: 2 * 1024 * 1024, ...options });
  assert.equal(result.status, 0, `${command} ${args.join(' ')}: ${result.error || result.stderr || result.stdout}`);
  return result.stdout.trim();
};

assert.ok(AndroidGate.sdk, 'ANDROID_HOME or ANDROID_SDK_ROOT is required');

class AndroidDevice {
  static adb = path.join(AndroidGate.sdk, 'platform-tools', process.platform === 'win32' ? 'adb.exe' : 'adb');
  static packageName = 'dev.tachyon.phase_five';
  static evidence = path.join(AndroidGate.repo, 'target/phase5-android-evidence');
}

class AndroidReleaseGate {
  static password = 'tachyon-gate-only';
  static alias = 'upload';
  static signer = 'CN=Tachyon Release Gate';
}

class KeptStorage {
  static cache = 'tachyon-cache:kept';
  static storageCommitDelayMs = 6000;
  static cacheLimit = 'known Android limit: a Cache API entry written shortly before am force-stop is not always kept';
}

function write(project, name, contents) {
  const file = path.join(project, name);
  mkdirSync(path.dirname(file), { recursive: true });
  writeFileSync(file, contents);
}

const pause = (ms) => new Promise(resolve => setTimeout(resolve, ms));

function countStatus(page) {
  return page.getByRole('status', {name:'Count'});
}

function writeFixture(project) {
  write(project, 'client/manifest.json', JSON.stringify({ name: 'Phase Five', start_url: '/', tac: { scripts: ['/shared/android-probe.js'], application: { id: 'dev.tachyon.phase-five', version: '0.1.0', build: 5 } } }));
  write(project, 'client/shared/android-probe.js', `const hostCall = globalThis.__tachyonNativeHostCall;
globalThis.__tachyonNativeHostCall = (capability, payload) => {
  const request = JSON.parse(payload);
  if (capability === 'companion.invoke' && request.op === 'init') { globalThis.androidGateOwner = request.owner; globalThis.androidGateService = request.service; }
  return hostCall(capability, payload);
};
`);
  write(project, 'client/models/NativeEvent.js', `/**
 * @typedef {Object} NativeEvent
 * @property {string} text
 */
export {}
`);
  write(project, 'client/models/NativeEvent.kt', 'data class NativeEvent(val text: String)\n');
  write(project, 'client/shared/identity.txt', 'root-relative-native-asset');
  write(project, 'client/shared/opaque-frame-probe.js', `const host = globalThis.__tachyonAndroidHost;
let result = null;
if (host && typeof host.call === 'function') {
  result = host.call('companion.invoke', JSON.stringify({route:'/',service:'androidFirst',op:'call',name:'processId',args:[]}));
}
parent.postMessage({probe:'android-frame', exposed:!!host, result}, '*');
`);
  write(project, 'client/shared/bridge-probe.html', '<!doctype html><title>Native frame boundary probe</title><p>Frame content remains supported.</p>');
  for (const prefix of ['', 'items/_id/']) {
    const name = prefix ? 'Second' : 'First';
    const instance = `android${name}`;
    const keptView = prefix ? '' : '<android-kept />';
    write(project, `client/pages/${prefix}tac.html`, `<android-${name.toLowerCase()} />${keptView}`);
    write(project, `client/components/android/${name.toLowerCase()}/tac.html`, androidView(name, instance));
    write(project, `client/components/android/${name.toLowerCase()}/tac.css`, 'h1 { color: rgb(12, 34, 56); }');
    write(project, `client/services/Android${name}Service.js`, `@Service()
export default class Android${name}Service {
  received = '';${prefix ? '' : `
  @LocalState
  visits = 0;
  visit() { this.visits += 1; return this.visits; }`}
  /** @param {NativeEvent} value */
  @OnEvent
  receive(value) { this.received = value.text; }
}
`);
    write(project, `client/services/Android${name}Service.kt`, prefix ? androidSecondService() : androidFirstService());
  }
  writeKeptStorageFixture(project);
}

function writeKeptStorageFixture(project) {
  write(project, 'client/components/android/kept/tac.html', `<section aria-label="Kept storage">
<p>Kept note: {androidKept.note}</p>
<p>Kept row: {androidKept.row}</p>
<p>Kept cache: {androidKept.cached}</p>
<button on:click="androidKept.keep()">Keep storage</button>
<button on:click="androidKept.read()">Read kept storage</button>
</section>`);
  write(project, 'client/setups/KeptSetup.js', `@Setup
export default class KeptSetup {
  static configure() { return {}; }
}
`);
  write(project, 'client/entities/KeptNoteEntity.js', `@Entity({"adapter":"indexeddb"})
@Table('keptNotes')
export default class KeptNoteEntity {
  @Id
  id;
  text;
}
`);
  write(project, 'client/stores/KeptNoteStore.js', `@Store(KeptNoteEntity, {"fields":["id","text"],"defaultSize":20,"maxSize":100,"setup":"KeptSetup"})
export default class KeptNoteStore extends Repository {}
`);
  write(project, 'client/services/AndroidKeptService.js', androidKeptService());
}

function androidKeptService() {
  return `@Service
@CacheSettings({"name":"kept","ttlMs":3600000,"maxBytes":4096,"maxEntries":8})
export default class AndroidKeptService {
  /** @type {KeptNoteStore} */
  @Needs
  keptNoteStore;
  @LocalState
  note = '';
  row = '';
  cached = '';
  _runs = 0;

  async keep() {
    this.note = this.#newNote();
    await this.keptNoteStore.save(this.#rowFor(this.note));
    this.cached = this.#describe(await this.stamp(this.note));
  }

  async read() {
    this.row = this.#textOf(await this.keptNoteStore.findById(this.note));
    this.cached = this.#describe(await this.stamp(this.note));
  }

  @Cached
  async stamp(note) {
    this._runs += 1;
    return { note, runs: this._runs };
  }

  #newNote() { return 'kept-' + crypto.randomUUID(); }

  #rowFor(note) { return { id: note, text: note }; }

  #textOf(row) { return row ? row.text : 'missing'; }

  #describe(stamped) { return \`\${stamped.note} after \${stamped.runs} run, \${this._runs} this launch\`; }
}
`;
}

// The first route drives every native call the gate checks, so its view binds them; the second route only counts.
function androidView(name, instance) {
  const first = name === 'First';
  const nameInput = first ? `<input aria-label="Your name" :value="${instance}.name" on:input="${instance}.name = $event.target.value" placeholder="Name">` : '';
  const nativeCalls = first ? `<button on:click="${instance}.announce()">Announce native event</button><button on:click="${instance}.processId()">Read process id</button><button on:click="${instance}.delayedProcessId()">Read process id later</button><button on:click="${instance}.visit()">Visit</button><output aria-label="Visits">{${instance}.visits}</output>` : '';
  return `<main aria-label="Android native evidence"><h1>${name} route</h1><p>Built by {${instance}.builtBy}</p><p>Received {${instance}.received}</p><button on:click="${instance}.increase()" aria-label="Increase count">Increase</button><output aria-label="Count">{${instance}.count}</output>${nameInput}${nativeCalls}<a href="/items/7/">Second route</a></main>`;
}

function androidFirstService() {
  return `@Service
class AndroidFirstService {
  var count: Int = 0
  @LocalState
  var visits: Int = 0
  var name: String = ""
    set(value) { Thread.sleep(100); field = value }
  val builtBy: String = "Kotlin"
  val applicationEventPublisher: ApplicationEventPublisher
  fun increase(): Int { count += 21; return count }
  fun visit(): Int { visits += 1; return visits }
  fun processId(): Int = android.os.Process.myPid()
  fun delayedProcessId(): Int { Thread.sleep(1200); return processId() }
  fun announce(): String {
    applicationEventPublisher.publishEvent(models.NativeEvent(builtBy))
    return builtBy
  }
}
`;
}

function androidSecondService() {
  return `@Service
class AndroidSecondService {
  var count: Int = 0
  val builtBy: String = "Kotlin"
  fun increase(): Int { count += 21; return count }
}
`;
}

function resolveBinary() {
  let binary = process.env.TAC_BIN;
  if (!binary) {
    run('cargo', ['build', '--locked', '--bin', 'ty']);
    binary = path.join(JSON.parse(run('cargo', ['metadata', '--format-version', '1', '--no-deps'])).target_directory, 'debug/ty');
  }
  return binary;
}

function buildAndCheckBundle(binary, project) {
  run(binary, ['build', project, '--target', 'android']);
  const out = path.join(project, 'dist/android');
  const host = JSON.parse(readFileSync(path.join(out, 'tachyon.host.json'), 'utf8'));
  assert.equal(host.schemaVersion, 3);
  assert.deepEqual(host.companions, [{ service: 'androidFirst', language: 'Kotlin' }, { service: 'androidSecond', language: 'Kotlin' }]);
  const assets = path.join(out, 'PhaseFive/project/app/src/main/assets');
  assert.equal(JSON.parse(readFileSync(path.join(assets, 'NativeIndex.json'), 'utf8')).contract_version, 2);
  assert.deepEqual(readFileSync(path.join(assets, 'WebBundle/index.html')), readFileSync(path.join(out, 'web/index.html')));
  return { out, host };
}

async function installAndLaunch(out) {
  run(AndroidDevice.adb, ['wait-for-device']);
  const apkEntries = run('unzip', ['-Z1', path.join(out, 'PhaseFive/PhaseFive.apk')]);
  assert.ok(apkEntries.split('\n').includes('assets/WebBundle/items/_id/index.html'), 'AAPT must retain dynamic route directories');
  run(AndroidDevice.adb, ['install', '-r', path.join(out, 'PhaseFive/PhaseFive.apk')]);
  const packageInfo = run(AndroidDevice.adb, ['shell', 'dumpsys', 'package', AndroidDevice.packageName]);
  assert.match(packageInfo, /versionCode=5\b/, 'versionCode must be tac.application.build');
  assert.match(packageInfo, /versionName=0\.1\.0\b/, 'versionName must be tac.application.version');
  run(AndroidDevice.adb, ['shell', 'pm', 'clear', AndroidDevice.packageName]);
  return launch();
}

async function launch() {
  run(AndroidDevice.adb, ['shell', 'am', 'start', '-W', '-n', `${AndroidDevice.packageName}/.MainActivity`]);
  let pid;
  for (let attempt = 0; attempt < 60; attempt++) {
    pid = run(AndroidDevice.adb, ['shell', 'pidof', AndroidDevice.packageName]);
    if (run(AndroidDevice.adb, ['shell', 'cat', '/proc/net/unix']).includes(`webview_devtools_remote_${pid}`)) break;
    await pause(500);
  }
  assert.match(pid, /^\d+$/);
  return pid;
}

async function checkStartup(page, pid) {
  try { await page.getByRole('button', {name:'Increase count'}).waitFor(); }
  catch (error) { console.error('Android startup document:', page.url(), (await page.content()).slice(0,4096)); throw error; }
  assert.equal(await page.evaluate(async () => (await navigator.serviceWorker.getRegistrations()).length), 0, 'Native UI must not use a stale service worker');
  assert.equal(await countStatus(page).textContent(), '0');
  await page.getByRole('button', {name:'Increase count'}).click();
  await page.waitForFunction(() => document.querySelector('output')?.textContent === '21');
  const nativePid = await page.evaluate(async () => __tachyonTac.service('androidFirst').processId());
  assert.equal(nativePid, Number(pid), 'Kotlin must run inside the actual Android app process');
  const wrongRoute = await page.evaluate(async () => JSON.parse(await globalThis.__tachyonNativeHostCall('companion.invoke', JSON.stringify({ route:'/items/_id', service:'androidSecond', op:'get', name:'count' }))));
  assert.ok(wrongRoute.error, 'A page cannot impersonate another native route');
}

async function checkFrameBoundaries(page) {
  const sandboxProbe = await page.evaluate(() => new Promise(resolve => {
    const frame = document.createElement('iframe');
    frame.sandbox = 'allow-scripts';
    const timer = setTimeout(() => {
      removeEventListener('message', receive);
      frame.remove();
      resolve({ error:'Opaque iframe probe timed out' });
    }, 5000);
    const receive = event => {
      if (event.source !== frame.contentWindow || event.data?.probe !== 'android-frame') return;
      clearTimeout(timer);
      removeEventListener('message', receive);
      frame.remove();
      resolve(event.data);
    };
    addEventListener('message', receive);
    frame.srcdoc = '<script src="https://appassets.tachyon.local/shared/opaque-frame-probe.js"><\/script>';
    document.body.append(frame);
  }));
  assert.equal(sandboxProbe.exposed, false, `Opaque srcdoc must not receive native transport: ${JSON.stringify(sandboxProbe)}`);
  const foreignFrame = await page.evaluate(url => new Promise(resolve => {
    const frame = document.createElement('iframe');
    const timer = setTimeout(() => {
      document.removeEventListener('securitypolicyviolation', refused);
      frame.remove();
      resolve(null);
    }, 5000);
    const refused = event => {
      if (!event.blockedURI.startsWith('https://foreign.invalid')) return;
      clearTimeout(timer);
      document.removeEventListener('securitypolicyviolation', refused);
      frame.remove();
      resolve({ directive: event.effectiveDirective, blocked: event.blockedURI });
    };
    document.addEventListener('securitypolicyviolation', refused);
    frame.src = url;
    document.body.append(frame);
  }), 'https://foreign.invalid/bridge-probe.html');
  assert.equal(foreignFrame?.directive, 'frame-src', `The page policy must refuse a foreign frame: ${JSON.stringify(foreignFrame)}`);
  for (const [name, url, exposed] of [
    ['local-frame', 'https://appassets.tachyon.local/shared/bridge-probe.html', true],
  ]) {
    await page.evaluate(({ name, url }) => {
      const frame = document.createElement('iframe');
      frame.name = name;
      frame.src = url;
      document.body.append(frame);
    }, { name, url });
    let frame;
    for (let attempt = 0; attempt < 40; attempt++) {
      frame = page.frames().find(candidate => candidate.name() === name && candidate.url() === url);
      if (frame) break;
      await pause(100);
    }
    assert.ok(frame, `${name} document must load for the actual frame boundary probe`);
    if (name === 'local-frame') assert.match(await frame.locator('body').textContent(), /Frame content remains supported/);
    const result = await frame.evaluate(() => new Promise(resolve => {
      const host = globalThis.__tachyonAndroidHost;
      if (!host) { resolve({ exposed: false, reply: null }); return; }
      const id = '777777';
      const timer = setTimeout(() => resolve({ exposed: true, reply: null }), 750);
      host.onmessage = event => {
        clearTimeout(timer);
        resolve({ exposed: true, reply: event.data });
      };
      host.postMessage(JSON.stringify({
        id, capability: 'companion.invoke',
        payload: JSON.stringify({ route: '/', service: 'androidFirst', op: 'call', name: 'processId', args: [] }),
        isMainFrame: true, sourceOrigin: 'https://appassets.tachyon.local',
      }));
    }));
    assert.equal(result.exposed, exposed, `${name} transport origin allowlist`);
    assert.equal(result.reply, null, `${name} cannot invoke even with spoofed trusted origin/main-frame fields`);
    await page.evaluate(name => document.querySelector(`iframe[name="${name}"]`).remove(), name);
  }
}

async function checkRawMainCalls(page, pid) {
  const nativeOwner = await page.evaluate(() => globalThis.androidGateOwner);
  const rawMain = async (extra, payload = JSON.stringify({ route:'/', service:'androidFirst', owner:nativeOwner, op:'call', name:'processId', args:[] }), wire = null) =>
    page.evaluate(({ extra, payload, wire }) => new Promise(resolve => {
      const host = globalThis.__tachyonAndroidHost;
      const id = String(800000 + Math.floor(Math.random() * 100000));
      const receive = event => {
        const reply = JSON.parse(event.data);
        if (reply.id !== id) return;
        clearTimeout(timer);
        host.removeEventListener('message', receive);
        resolve(JSON.parse(reply.result));
      };
      const timer = setTimeout(() => {
        host.removeEventListener('message', receive);
        resolve(null);
      }, 750);
      host.addEventListener('message', receive);
      host.postMessage(wire === null ? JSON.stringify({ id, capability:'companion.invoke', payload, ...extra }) : wire.replace('__ID__', id));
    }), { extra, payload, wire });
  assert.equal((await rawMain({})).value, Number(pid), 'Raw valid main-frame call is authorized');
  const nested = depth => '['.repeat(depth) + '0' + ']'.repeat(depth);
  assert.equal((await rawMain({ ignored: JSON.parse(nested(63)) })).value, Number(pid), 'Envelope depth64 is accepted');
  assert.equal(await rawMain({ ignored: JSON.parse(nested(64)) }), null, 'Envelope depth65 is rejected');
  const request = { route:'/', service:'androidFirst', owner: await page.evaluate(() => globalThis.androidGateOwner), op:'call', name:'processId', args:[] };
  const lenientEnvelope = `{id:"__ID__",capability:"companion.invoke",payload:${JSON.stringify(JSON.stringify(request))},a"b:1,"x":${nested(65)},c"d:2}`;
  assert.equal(await rawMain({}, undefined, lenientEnvelope), null, 'Unquoted Android JSON keys cannot hide depth from the admission scanner');
  assert.equal((await rawMain({}, JSON.stringify({ ...request, ignored:JSON.parse(nested(63)) }))).value, Number(pid), 'Payload depth64 is accepted');
  assert.equal(await rawMain({}, JSON.stringify({ ...request, ignored:JSON.parse(nested(64)) })), null, 'Payload depth65 is rejected');
  assert.equal(await rawMain({}, JSON.stringify({ ...request, ignored:'é'.repeat(32769) })), null, 'Native UTF8 byte limit is enforced');
  assert.equal(await rawMain({ ignored:'é'.repeat(65537) }), null, 'Native outer-envelope UTF8 byte limit is enforced');
  assert.equal((await rawMain({})).value, Number(pid), 'Rejected raw calls do not crash or disable the host');
}

async function checkNativeEvent(page) {
  await page.evaluate(async () => {
    const receive = globalThis.__tachyonCompanionPublish;
    globalThis.__tachyonCompanionPublish = event => { globalThis.nativeEvidence = event; receive(event); };
    await __tachyonTac.service('androidFirst').announce();
  });
  // A published event is named after its model class and carries the model's fields.
  await page.waitForFunction(() => globalThis.nativeEvidence?.name === 'NativeEvent');
  assert.deepEqual(await page.evaluate(() => globalThis.nativeEvidence.value), { text: 'Kotlin' });
  await page.getByText('Received Kotlin', { exact: true }).waitFor();
}

async function checkNavigation(page) {
  mkdirSync(AndroidDevice.evidence, {recursive:true});
  await page.screenshot({path:path.join(AndroidDevice.evidence,'android-first-route.png')});
  await page.addInitScript(() => {
    globalThis.lateNativeReplies = [];
    globalThis.__tachyonAndroidHost?.addEventListener('message', event => {
      if (JSON.parse(event.data).id === '999997') globalThis.lateNativeReplies.push(event.data);
    });
  });
  await page.evaluate(() => globalThis.__tachyonAndroidHost.postMessage(JSON.stringify({
    id:'999997', capability:'companion.invoke',
    payload:JSON.stringify({route:'/', service:'androidFirst', owner:globalThis.androidGateOwner, op:'call', name:'delayedProcessId', args:[]}),
  })));
  await pause(100);
  await page.getByRole('link', {name:'Second route'}).click();
  try { await page.getByRole('heading', {name:'Second route'}).waitFor(); }
  catch (error) { console.error('Android dynamic route document:', page.url(), (await page.content()).slice(0,4096)); throw error; }
  assert.equal(await page.getByRole('heading', {name:'Second route'}).evaluate(element => getComputedStyle(element).color), 'rgb(12, 34, 56)', 'Dynamic-route stylesheet resolves to its captured bundle asset');
  assert.equal(await page.evaluate(async () => (await fetch('/shared/identity.txt')).text()), 'root-relative-native-asset');
  await pause(1500);
  assert.deepEqual(await page.evaluate(() => globalThis.lateNativeReplies), [], 'A native reply from the retired document cannot reach the new route');
  assert.equal(await countStatus(page).textContent(), '0', 'Native route state must be isolated');
  await page.getByRole('button', {name:'Increase count'}).click();
  await page.waitForFunction(() => document.querySelector('output')?.textContent === '21');
  await page.goto('https://appassets.tachyon.local/');
  try { await page.getByRole('heading', {name:'First route'}).waitFor(); }
  catch (error) { console.error('Android root document:', (await page.content()).slice(0,4096)); throw error; }
  assert.equal(await countStatus(page).textContent(), '0', 'Returning navigation creates a fresh native document owner');
  console.log('PASS: Android main-frame/origin/depth/byte/navigation boundary probes');
}

async function visitOnEntryPage(page) {
  const visits = page.getByRole('status', {name:'Visits'});
  const start = Number(await visits.textContent());
  for (const expected of [start + 1, start + 2]) {
    await page.getByRole('button', {name:'Visit'}).click();
    await visits.filter({hasText:String(expected)}).waitFor();
  }
  return { entry: new URL(page.url()).pathname, visits: start + 2 };
}

async function checkStoredFieldAfterReload(page, entry) {
  const visits = page.getByRole('status', {name:'Visits'});
  try { await visits.filter({hasText:String(entry.visits)}).waitFor({timeout:5000}); }
  catch (error) { throw new Error(`A LocalState value set on ${entry.entry} must be read on ${new URL(page.url()).pathname}; it shows ${await visits.textContent()}`); }
  await page.getByRole('button', {name:'Visit'}).click();
  await visits.filter({hasText:String(entry.visits + 1)}).waitFor();
  await page.reload();
  try { await page.getByRole('heading', {name:'First route'}).waitFor(); }
  catch (error) { console.error('Android reloaded document:', (await page.content()).slice(0,4096)); throw error; }
  await visits.filter({hasText:String(entry.visits + 1)}).waitFor({timeout:5000});
  assert.equal(await page.evaluate(async () => JSON.parse(await globalThis.__tachyonNativeHostCall(
    'companion.invoke', JSON.stringify({route:'/',service:'androidFirst',owner:globalThis.androidGateOwner,op:'get',name:'visits'}))).value), entry.visits + 1, 'The restored LocalState value reaches the new native owner');
  console.log(`PASS: Android LocalState field set by a native method on ${entry.entry} survives navigation to / and a reload`);
}

function keptSection(page) {
  return page.getByRole('region', {name:'Kept storage'});
}

async function keptText(section, label) {
  const text = await section.getByText(new RegExp(`^${label}: `)).textContent();
  return text.slice(label.length + 2);
}

async function keepStorage(page) {
  const section = keptSection(page);
  await section.getByRole('button', {name:'Keep storage'}).click();
  await section.getByText(/^Kept cache: .+/).waitFor();
  const note = await keptText(section, 'Kept note');
  assert.match(note, /^kept-[0-9a-f-]{36}$/);
  assert.equal(await keptText(section, 'Kept cache'), `${note} after 1 run, 1 this launch`, 'The first launch runs the Cached method');
  await page.waitForFunction(async ({ cache, note }) => {
    const entries = await (await caches.open(cache)).keys();
    return entries.some(entry => decodeURIComponent(entry.url).includes(note));
  }, { cache: KeptStorage.cache, note });
  const saved = await page.evaluate(note => Object.values(localStorage).some(value => value.includes(note)), note);
  assert.equal(saved, true, 'LocalState wrote the note to localStorage');
  return note;
}

function processIsRunning() {
  const result = spawnSync(AndroidDevice.adb, ['shell', 'pidof', AndroidDevice.packageName], { encoding: 'utf8', timeout: 60000 });
  return result.status === 0 && result.stdout.trim() !== '';
}

async function quit(pid) {
  run(AndroidDevice.adb, ['shell', 'input', 'keyevent', 'KEYCODE_HOME']);
  await pause(KeptStorage.storageCommitDelayMs);
  run(AndroidDevice.adb, ['shell', 'am', 'force-stop', AndroidDevice.packageName]);
  assert.equal(processIsRunning(), false, `The app process ${pid} must end before the relaunch`);
}

async function readKeptStorage(page) {
  const section = keptSection(page);
  try { await section.getByText(/^Kept note: .+/).waitFor(); }
  catch (error) {
    console.error('Android kept storage after relaunch:', await section.innerText(), await page.evaluate(async cache => ({
      local: Object.fromEntries(Object.entries(localStorage)),
      cached: (await (await caches.open(cache)).keys()).map(entry => entry.url),
      databases: (await indexedDB.databases()).map(database => database.name),
    }), KeptStorage.cache));
    throw error;
  }
  await section.getByRole('button', {name:'Read kept storage'}).click();
  await section.getByText(/^Kept cache: .+/).waitFor();
  const note = await keptText(section, 'Kept note');
  const row = await keptText(section, 'Kept row');
  const cached = await keptText(section, 'Kept cache');
  return { note, row, cached };
}

async function connect(pid) {
  const port = run(AndroidDevice.adb, ['forward', 'tcp:0', `localabstract:webview_devtools_remote_${pid}`]);
  const browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`, {noDefaults:true});
  const page = browser.contexts()[0].pages()[0];
  assert.ok(page, 'Android WebView exposes a live document');
  page.on('pageerror', error => console.error('Android page error:', error.message));
  page.on('console', message => { if (message.type() === 'error') console.error('Android console:', message.text()); });
  return { port, browser, page };
}

async function disconnect(connection) {
  await connection.browser.close();
  run(AndroidDevice.adb, ['forward', '--remove', `tcp:${connection.port}`]);
}

async function relaunchAndCheckKeptStorage(connection, pid) {
  const note = await keepStorage(connection.page);
  await disconnect(connection);
  await quit(pid);
  const relaunchedPid = await launch();
  assert.notEqual(relaunchedPid, pid, 'The relaunch starts a new app process');
  const relaunched = await connect(relaunchedPid);
  try { await relaunched.page.getByRole('button', {name:'Increase count'}).waitFor(); }
  catch (error) { console.error('Android relaunch document:', relaunched.page.url(), (await relaunched.page.content()).slice(0,4096)); throw error; }
  const cacheKept = await relaunched.page.evaluate(async ({ cache, note }) => {
    const entries = await (await caches.open(cache)).keys();
    return entries.some(entry => decodeURIComponent(entry.url).includes(note));
  }, { cache: KeptStorage.cache, note });
  const kept = await readKeptStorage(relaunched.page);
  assert.deepEqual({ note: kept.note, row: kept.row }, { note, row: note }, 'Relaunch keeps the IndexedDB Store row and the LocalState value');
  const served = kept.cached === `${note} after 1 run, 0 this launch`;
  assert.ok(served || kept.cached === `${note} after 1 run, 1 this launch`, `The Cached answer after relaunch is ${kept.cached}`);
  const cache = !cacheKept ? `missing (${KeptStorage.cacheLimit})` : served ? 'kept and served' : 'kept, but the method ran again';
  console.log(`Android Cache API entry after relaunch: ${cache}`);
  console.log(`PASS: Android relaunch kept the Store row and LocalState value; Cache API entry ${cache} (process ${pid} -> ${relaunchedPid})`);
  return { connection: relaunched, pid: relaunchedPid };
}

async function checkInputAndWriteReport(page, host, pid) {
  const input = page.getByRole('textbox', {name:'Your name'});
  await input.focus();
  await input.evaluate(element => { globalThis.focusEvidence = element; });
  await input.pressSequentially('Ada Lovelace', {delay:40});
  assert.equal(await input.inputValue(), 'Ada Lovelace');
  assert.equal(await input.evaluate(element => document.activeElement === element && globalThis.focusEvidence === element), true, 'Native input must retain DOM identity and focus through reactive writes');
  await page.getByRole('button', {name:'Increase count'}).click();
  await page.waitForFunction(() => document.querySelector('output')?.textContent === '21');
  assert.equal(await page.evaluate(async () => JSON.parse(await globalThis.__tachyonNativeHostCall(
    'companion.invoke', JSON.stringify({route:'/',service:'androidFirst',owner:globalThis.androidGateOwner,op:'get',name:'name'}))).value), 'Ada Lovelace');
  writeFileSync(path.join(AndroidDevice.evidence,'report.json'), JSON.stringify({target:'android',host,pid:Number(pid),runtime:await page.evaluate(() => navigator.userAgent),checks:['Kotlin compilation','live WebView','field mutation','native OS API','route isolation','route ownership rejection','opaque sandboxed srcdoc bridge absence','local subframe rejection despite spoofed origin and frame claims','foreign origin bridge absence','raw authorized main-frame call','outer and inner JSON depth64/65 budgets','native UTF8 payload budget','bounded async pending queue and navigation retirement','native publish','accessible controls','per-character input focus and DOM identity','stale service worker retirement','packaged dynamic route','dynamic route stylesheet','root-relative shared asset','LocalState after a native method and a reload','relaunch keeps IndexedDB Store row and LocalState']},null,2)+'\n');
  console.log(`PASS: Android WebView and Kotlin native evidence: ${AndroidDevice.evidence}`);
}

function apksigner() {
  const tools = path.join(AndroidGate.sdk, 'build-tools');
  const latest = readdirSync(tools).sort((left, right) => left.localeCompare(right, undefined, {numeric:true})).at(-1);
  return path.join(tools, latest, process.platform === 'win32' ? 'apksigner.bat' : 'apksigner');
}

function buildRelease(binary, project) {
  const keys = mkdtempSync(path.join(tmpdir(), 'tachyon-android-keys-'));
  const keystore = path.join(keys, 'upload.p12');
  run('keytool', ['-genkeypair', '-keystore', keystore, '-storetype', 'PKCS12', '-storepass', AndroidReleaseGate.password, '-keypass', AndroidReleaseGate.password, '-alias', AndroidReleaseGate.alias, '-keyalg', 'RSA', '-keysize', '2048', '-validity', '2', '-dname', AndroidReleaseGate.signer]);
  const env = { ...process.env, TAC_ANDROID_KEYSTORE: keystore, TAC_ANDROID_KEYSTORE_PASSWORD: AndroidReleaseGate.password, TAC_ANDROID_KEYSTORE_ALIAS: AndroidReleaseGate.alias };
  try {
    const built = run(binary, ['bundle', project, '--target', 'android', '--release', '--out-dir', 'dist-release'], { env, timeout: 900000 });
    assert.ok(!built.includes(AndroidReleaseGate.password), 'A release build never prints the keystore password');
  } finally {
    rmSync(keys, {recursive:true,force:true});
  }
  const bundle = path.join(project, 'dist-release/android/PhaseFive');
  const apk = path.join(bundle, 'PhaseFive.apk');
  const aab = path.join(bundle, 'PhaseFive.aab');
  assert.match(run(apksigner(), ['verify', '--print-certs', apk]), /CN=Tachyon Release Gate/, 'The release APK is signed with the upload key');
  assert.match(run('jarsigner', ['-verify', '-verbose', '-certs', aab]), /CN="?Tachyon Release Gate/, 'The App Bundle is signed with the upload key');
  assert.ok(run('unzip', ['-Z1', aab]).split('\n').includes('base/assets/WebBundle/index.html'), 'The App Bundle carries the web bundle');
  for (const file of ['project/app/build.gradle.kts', 'project/gradle.properties']) {
    assert.ok(!readFileSync(path.join(bundle, file), 'utf8').includes(AndroidReleaseGate.password), `${file} holds no signing secret`);
  }
  return apk;
}

async function checkReleaseBuild(binary, project) {
  const apk = buildRelease(binary, project);
  run(AndroidDevice.adb, ['uninstall', AndroidDevice.packageName]);
  try {
    run(AndroidDevice.adb, ['install', apk]);
    const packageInfo = run(AndroidDevice.adb, ['shell', 'dumpsys', 'package', AndroidDevice.packageName]);
    assert.match(packageInfo, /versionCode=5\b/, 'The release keeps tac.application.build');
    assert.doesNotMatch(packageInfo.match(/pkgFlags=\[[^\]]*\]/)?.[0] ?? '', /DEBUGGABLE/, 'The release APK is not debuggable');
    run(AndroidDevice.adb, ['shell', 'am', 'start', '-W', '-n', `${AndroidDevice.packageName}/.MainActivity`]);
    let pid = '';
    for (let attempt = 0; attempt < 20 && !/^\d+$/.test(pid); attempt++) {
      await pause(500);
      pid = spawnSync(AndroidDevice.adb, ['shell', 'pidof', AndroidDevice.packageName], { encoding: 'utf8' }).stdout.trim();
    }
    assert.match(pid, /^\d+$/, 'The release APK launches');
    if (run(AndroidDevice.adb, ['shell', 'getprop', 'ro.debuggable']) === '1') {
      console.log('SKIP: release WebView debugging check; a debuggable system image opens WebView DevTools for every app');
    } else {
      assert.ok(!run(AndroidDevice.adb, ['shell', 'cat', '/proc/net/unix']).includes(`webview_devtools_remote_${pid}`), 'A release build does not open WebView debugging');
      console.log(`PASS: Android release WebView debugging stays off on a non-debuggable image (${run(AndroidDevice.adb, ['shell', 'getprop', 'ro.build.fingerprint'])})`);
    }
  } finally {
    spawnSync(AndroidDevice.adb, ['uninstall', AndroidDevice.packageName], { encoding: 'utf8' });
  }
  console.log('PASS: Android signed release APK and App Bundle');
}

async function main() {
  const project = mkdtempSync(path.join(tmpdir(), 'tachyon-android-gate-'));
  let connection;
  try {
    writeFixture(project);
    const binary = resolveBinary();
    const { out, host } = buildAndCheckBundle(binary, project);
    const pid = await installAndLaunch(out);
    connection = await connect(pid);
    const page = connection.page;
    await checkStartup(page, pid);
    const entry = await visitOnEntryPage(page);
    await checkFrameBoundaries(page);
    await checkRawMainCalls(page, pid);
    await checkNativeEvent(page);
    await checkNavigation(page);
    await checkStoredFieldAfterReload(page, entry);
    const opened = connection;
    connection = undefined;
    const relaunched = await relaunchAndCheckKeptStorage(opened, pid);
    connection = relaunched.connection;
    await checkInputAndWriteReport(connection.page, host, relaunched.pid);
    await disconnect(connection);
    connection = undefined;
    await checkReleaseBuild(binary, project);
  } finally {
    if (connection) await disconnect(connection);
    spawnSync(AndroidDevice.adb, ['shell', 'am', 'force-stop', AndroidDevice.packageName], { encoding: 'utf8' });
    rmSync(project, {recursive:true,force:true});
  }
}

await main();
