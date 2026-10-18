#!/usr/bin/env node
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

assert.equal(process.platform, 'darwin', 'iOS qualification requires macOS and Xcode');
class IosGate {
  static repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
  static project = mkdtempSync(path.join(tmpdir(), 'tachyon-ios-gate-'));
  static deviceSet = path.join(IosGate.project, 'simulator-devices');
  static appId = 'dev.tachyon.ios-gate';
  static evidence = path.join(IosGate.repo, 'target/phase5-ios-evidence');
  static device;
}

const run = (program, args, timeout = 120000) => {
  const result = spawnSync(program, args, {
    cwd: IosGate.repo, encoding: 'utf8', timeout, maxBuffer: 2 * 1024 * 1024,
  });
  assert.equal(result.status, 0, program + ' ' + args.join(' ') + ': ' +
    (result.error || result.stderr || result.stdout));
  return result.stdout.trim();
};
const simctl = (...args) => run('/usr/bin/xcrun', ['simctl', '--set', IosGate.deviceSet, ...args]);
const write = (relative, source) => {
  const file = path.join(IosGate.project, relative);
  mkdirSync(path.dirname(file), { recursive: true });
  writeFileSync(file, source);
};
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const cleanup = () => {
  if (IosGate.device) {
    for (const args of [['terminate', IosGate.device, IosGate.appId], ['shutdown', IosGate.device], ['delete', IosGate.device]]) {
      spawnSync('/usr/bin/xcrun', ['simctl', '--set', IosGate.deviceSet, ...args], { timeout: 30000, stdio: 'ignore' });
    }
    IosGate.device = undefined;
  }
  rmSync(IosGate.project, { recursive: true, force: true });
};
process.once('SIGINT', () => { cleanup(); process.exit(130); });
process.once('SIGTERM', () => { cleanup(); process.exit(143); });

function writeFixture() {
  mkdirSync(IosGate.deviceSet);
  write('client/manifest.json', JSON.stringify({ name: 'NativeIOSGate', start_url: '/', tac: { application: { id: 'dev.tachyon.ios-gate', version: '0.0.1', build: 4 }, styles: ['/shared/ios-gate.css'], names: { check: false }, constants: { check: false }, visibility: { check: false }, duplication: { check: false }, cohesion: { check: false } } }));
  write('client/models/IosEvent.js', '/**\n * @typedef {Object} IosEvent\n * @property {string} text\n */\nexport {}\n');
  write('client/models/IosEvent.swift', 'struct IosEvent {\n    let text: String\n}\n');
  write('client/shared/ios-gate.js', `export const marker = 'Shared module ready';
const hostCall = globalThis.__tachyonNativeHostCall;
globalThis.__tachyonNativeHostCall = (capability, payload) => {
  const request = JSON.parse(payload);
  if (capability === 'companion.invoke' && request.op === 'init' && request.service && !globalThis.iosGateService) { globalThis.iosGateOwner = request.owner; globalThis.iosGateService = request.service; }
  return hostCall(capability, payload);
};\n`);
  write('client/shared/ios-gate.css', 'body { --ios-gate: ready; font-family: sans-serif; }\n');
  for (const [prefix, label, value] of [['', 'root', 7], ['items/_id/', 'second', 9]]) {
    const service = 'Ios' + label[0].toUpperCase() + label.slice(1) + 'Service';
    const instance = 'ios' + label[0].toUpperCase() + label.slice(1);
    write('client/pages/' + prefix + 'tac.html', `<ios-${label} />`);
    write('client/components/ios/' + label + '/tac.html', `<main><h1 id="heading">${label} route</h1><p id="marker">{${instance}.marker}</p>
<p id="count">{${instance}.count}</p><p id="signal">{${instance}.signal}</p>
<button id="update" on:click="${instance}.count = ${value}">Update native</button>
<input id="name" :value="${instance}.name" on:input="${instance}.name = $event.target.value" aria-label="Name">
<a id="next" href="${prefix ? '/?returned=1' : '/items/7/'}">Next route</a></main>`);
    write('client/services/' + service + '.js', String.raw`import { marker } from '/shared/ios-gate.js';
@Service()
export default class ` + service + ' {\n  pageLabel = ' + JSON.stringify(label) + ';\n' + String.raw`  marker = marker;
  name = '';
  signal = 'waiting';
  @LocalState
  saved = '';
  /** @param {IosEvent} value */
  @OnEvent
  receive(value) { this.signal = value.text; }
  @OnStart()
  initialize() {
    setTimeout(() => { void this.probe(); }, 50);
  }
  async until(check) {
    for (let attempt = 0; attempt < 100; attempt++) {
      if (check()) return;
      await new Promise(resolve => setTimeout(resolve, 50));
    }
    throw new Error('Rendered iOS evidence did not become ready');
  }
  async probe() {
    try {
      await this.until(() => document.querySelector('#heading')?.textContent === this.pageLabel + ' route');
      const returned = this.pageLabel === 'root' && new URLSearchParams(location.search).has('returned');
      if (this.count !== 0) throw new Error('Native document owner state leaked');
      if (this.pageLabel === 'root' && this.saved !== (returned ? 'kept' : '')) throw new Error('LocalState after a native method and a page load: ' + JSON.stringify({ returned, saved: this.saved, stored: Object.entries(localStorage), path: location.href }));
      if (this.pageLabel === 'root' && !returned && await this.remember() !== 'kept') throw new Error('Native LocalState method');
      if (document.querySelector('#marker').textContent !== 'Shared module ready') throw new Error('Shared JS');
      await this.until(() => getComputedStyle(document.body).getPropertyValue('--ios-gate').trim() === 'ready');
      document.querySelector('#update').click();
      const expected = this.pageLabel === 'root' ? 7 : 9;
      await this.until(() => document.querySelector('#count')?.textContent === String(expected));
      if (await this.doubled() !== expected * 2) throw new Error('Native method');
      const input = document.querySelector('#name');
      input.focus();
      input.value = 'Ada Lovelace';
      input.dispatchEvent(new Event('input', { bubbles: true }));
      await this.until(() => this.name === 'Ada Lovelace');
      await new Promise(resolve => setTimeout(resolve, 100));
      if (document.querySelector('#name') !== input || document.activeElement !== input) throw new Error('Input focus');
      let routeRejected = false;
      try {
        const rejected = JSON.parse(await __tachyonNativeHostCall(
          'companion.invoke', JSON.stringify({ route:'/not-this-route', op:'init' })));
        routeRejected = Boolean(rejected.error);
      } catch { routeRejected = true; }
      if (!routeRejected) throw new Error('Native route impersonation accepted');
      const ownRoute = this.pageLabel === 'root' ? '/' : '/items/_id';
      const otherRoute = this.pageLabel === 'root' ? '/items/_id' : '/';
      for (const spelling of ['route', '\\u0072oute']) {
        let duplicateRejected = false;
        const ambiguous = '{"route":' + JSON.stringify(ownRoute) + ',"' + spelling + '":' +
          JSON.stringify(otherRoute) + ',"op":"get","name":"count"}';
        try {
          duplicateRejected = Boolean(JSON.parse(await __tachyonNativeHostCall(
            'companion.invoke', ambiguous)).error);
        } catch { duplicateRejected = true; }
        if (!duplicateRejected) throw new Error('Duplicate route key crossed the native guard');
      }
      const unicode = JSON.parse(await __tachyonNativeHostCall('companion.invoke',
        JSON.stringify({ '\u0301': '😀', route: ownRoute, service: iosGateService, owner: iosGateOwner, op: 'get', name: 'count' })));
      if (unicode.value !== expected) throw new Error('Unicode JSON failed at native boundary');
      await this.announce();
      await this.until(() => document.querySelector('#signal')?.textContent === 'received');
      this.phase = returned ? 'returned' : 'initial';
      this.proof = JSON.stringify({ route: this.pageLabel, count: this.count,
        doubled: await this.doubled(), nativePid: await this.processId(),
        sharedAssets: true, renderedInteraction: true, focusPreserved: true,
        routeRejected: true, duplicateRouteRejected: true, unicodeBoundaryPassed: true,
        publishReceived: true, saved: this.saved });
      await this.record();
      if (!returned) document.querySelector('#next').click();
    } catch (error) {
      this.phase = 'error';
      this.proof = JSON.stringify({ error: String(error) });
      await this.record();
    }
  }
}
`);
    write('client/services/' + service + '.swift', `import Foundation
@Service
final class ${service} {
    var count: Int = 0
    var phase: String = "initial"
    var proof: String = ""
    var saved: String = ""
    func remember() -> String {
        saved = "kept"
        return saved
    }
    private let applicationEventPublisher: ApplicationEventPublisher
    func doubled() -> Int { count * 2 }
    func processId() -> Int { Int(ProcessInfo.processInfo.processIdentifier) }
    func announce() -> String {
        applicationEventPublisher.publishEvent(IosEvent(text: "received"))
        return "received"
    }
    func record() -> Int {
        let directory = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
        let safePhase = ["initial", "returned", "error"].contains(phase) ? phase : "error"
        let destination = directory.appendingPathComponent("${label}-" + safePhase + ".json")
        let record: [String: Any] = ["pid": Int(ProcessInfo.processInfo.processIdentifier),
                                    "count": count, "proof": proof]
        if let bytes = try? JSONSerialization.data(withJSONObject: record, options: [.sortedKeys]) {
            try? bytes.write(to: destination, options: .atomic)
        }
        return Int(ProcessInfo.processInfo.processIdentifier)
    }
}
`);
  }
}

function resolveBinary() {
  let binary = process.env.TAC_BIN;
  if (!binary) {
    run('cargo', ['build', '--locked', '--bin', 'ty'], 240000);
    const metadata = JSON.parse(run('cargo', ['metadata', '--format-version', '1', '--no-deps']));
    binary = path.join(metadata.target_directory, 'debug/ty');
  }
  return binary;
}

function buildIosApp(binary) {
  console.log('Building actual iOS host with two Swift services...');
  run(binary, ['build', IosGate.project, '--target', 'ios'], 240000);
  const output = path.join(IosGate.project, 'dist/ios');
  const bundle = path.join(output, 'NativeIOSGate.app');
  const manifest = JSON.parse(readFileSync(path.join(output, 'tachyon.host.json'), 'utf8'));
  assert.equal(manifest.schemaVersion, 3);
  assert.deepEqual(manifest.companions, [
    { service: 'iosRoot', language: 'Swift' }, { service: 'iosSecond', language: 'Swift' },
  ]);
  assert.deepEqual(readFileSync(path.join(output, 'web/index.html')),
    readFileSync(path.join(bundle, 'WebBundle/index.html')));
  const plist = path.join(bundle, 'Info.plist');
  assert.equal(run('/usr/bin/plutil', ['-extract', 'CFBundleShortVersionString', 'raw', plist]), '0.0.1');
  assert.equal(run('/usr/bin/plutil', ['-extract', 'CFBundleVersion', 'raw', plist]), '4');
  return bundle;
}

function bootSimulator() {
  const runtimes = JSON.parse(simctl('list', 'runtimes', '--json')).runtimes;
  const runtime = runtimes.filter(item => item.isAvailable && item.identifier.includes('.iOS-'))
    .sort((a, b) => b.version.localeCompare(a.version, undefined, { numeric: true }))[0];
  assert.ok(runtime, 'an installed iOS simulator runtime is required');
  const types = runtime.supportedDeviceTypes ??
    JSON.parse(simctl('list', 'devicetypes', '--json')).devicetypes;
  const phone = types.find(item => item.name === 'iPhone SE (3rd generation)') ??
    types.find(item => item.productFamily === 'iPhone');
  assert.ok(phone, 'the installed runtime needs an available iPhone device type');
  IosGate.device = simctl('create', 'TachyonGate-' + randomUUID(), phone.identifier, runtime.identifier);
  assert.match(IosGate.device, /^[A-Fa-f0-9-]{36}$/);
  console.log('Booting isolated ' + phone.name + ' with iOS ' + runtime.version + '...');
  simctl('boot', IosGate.device);
  run('/usr/bin/xcrun', ['simctl', '--set', IosGate.deviceSet, 'bootstatus', IosGate.device, '-b'], 240000);
  return { runtime, phone };
}

function launchApp(bundle) {
  simctl('install', IosGate.device, bundle);
  const launch = simctl('launch', IosGate.device, IosGate.appId);
  const pid = Number(launch.match(/:\s*(\d+)\s*$/)?.[1]);
  assert.ok(Number.isSafeInteger(pid) && pid > 0, 'simctl must report the actual app PID');
  return pid;
}

async function waitForProofFiles() {
  const container = simctl('get_app_container', IosGate.device, IosGate.appId, 'data');
  const documents = path.join(container, 'Documents');
  const required = ['root-initial.json', 'second-initial.json', 'root-returned.json'];
  const deadline = Date.now() + 60000;
  while (Date.now() < deadline) {
    for (const label of ['root', 'second']) {
      const failure = path.join(documents, label + '-error.json');
      assert.ok(!existsSync(failure), existsSync(failure) ? readFileSync(failure, 'utf8') : '');
    }
    if (required.every(name => existsSync(path.join(documents, name)))) break;
    await pause(250);
  }
  return { documents, required };
}

function collectProofs(documents, required, pid) {
  const proofs = required.map(name => {
    const file = path.join(documents, name);
    assert.ok(existsSync(file), 'real iOS WebView did not produce ' + name);
    const native = JSON.parse(readFileSync(file, 'utf8'));
    const proof = JSON.parse(native.proof);
    assert.equal(native.pid, pid, 'proof must originate in the launched simulator app');
    assert.equal(proof.nativePid, pid, 'JavaScript must call into that native process');
    assert.equal(native.count, name.startsWith('second') ? 9 : 7);
    assert.equal(proof.count, native.count);
    assert.equal(proof.doubled, native.count * 2);
    for (const key of ['sharedAssets', 'renderedInteraction', 'focusPreserved', 'routeRejected',
      'duplicateRouteRejected', 'unicodeBoundaryPassed', 'publishReceived']) {
      assert.equal(proof[key], true, name + ': ' + key);
    }
    assert.equal(proof.saved, name.startsWith('root') ? 'kept' : '', name + ': LocalState after a native method');
    return { file: name, ...native, proof };
  });
  return proofs;
}

async function main() {
  try {
    run(process.execPath, [path.join(IosGate.repo, 'scripts/native/apple-protocol-test.mjs')], 90000);
    writeFixture();
    const binary = resolveBinary();
    const bundle = buildIosApp(binary);

    const { runtime, phone } = bootSimulator();
    const pid = launchApp(bundle);
    const { documents, required } = await waitForProofFiles();
    mkdirSync(IosGate.evidence, { recursive: true });
    const proofs = collectProofs(documents, required, pid);
    const screenshot = path.join(IosGate.project, 'ios-returned-root.png');
    simctl('io', IosGate.device, 'screenshot', screenshot);
    copyFileSync(screenshot, path.join(IosGate.evidence, 'ios-returned-root.png'));
    writeFileSync(path.join(IosGate.evidence, 'report.json'), JSON.stringify({
      runtime: runtime.version, deviceType: phone.name, binary, pid, proofs,
    }, null, 2) + '\n');
    console.log('PASS: real iOS WKWebView render/input/focus, Swift ABI/OS/publish, shared assets, dynamic routes, isolation, and LocalState kept after a native method and a page load');
  } finally {
    cleanup();
  }
}

await main();
