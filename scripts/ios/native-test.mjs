#!/usr/bin/env node
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { homedir, tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { crc32, deflateSync } from 'node:zlib';
import { RelaunchStorageProbe } from '../native/relaunch-storage-probe.mjs';

assert.equal(process.platform, 'darwin', 'iOS qualification requires macOS and Xcode');
class IosGate {
  static repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
  static project = mkdtempSync(path.join(tmpdir(), 'tachyon-ios-gate-'));
  static chosenDevice = process.env.TAC_IOS_DEVICE ?? '';
  static deviceSet = process.env.TAC_IOS_DEVICE_SET ??
    (IosGate.chosenDevice ? path.join(homedir(), 'Library/Developer/CoreSimulator/Devices') : path.join(IosGate.project, 'simulator-devices'));
  static installMilliseconds = 900000;
  static appId = 'dev.tachyon.ios-gate';
  static evidence = path.join(IosGate.repo, 'target/phase5-ios-evidence');
  static storageProof = 'root-initial.json';
  static runtimeVersion = process.env.TAC_IOS_RUNTIME ?? '';
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
const slowSimctl = (...args) => run('/usr/bin/xcrun', ['simctl', '--set', IosGate.deviceSet, ...args], IosGate.installMilliseconds);
const write = (relative, source) => {
  const file = path.join(IosGate.project, relative);
  mkdirSync(path.dirname(file), { recursive: true });
  writeFileSync(file, source);
};
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const cleanup = () => {
  if (IosGate.device) {
    const steps = IosGate.chosenDevice
      ? [['terminate', IosGate.device, IosGate.appId], ['uninstall', IosGate.device, IosGate.appId]]
      : [['terminate', IosGate.device, IosGate.appId], ['shutdown', IosGate.device], ['delete', IosGate.device]];
    for (const args of steps) {
      spawnSync('/usr/bin/xcrun', ['simctl', '--set', IosGate.deviceSet, ...args], { timeout: 30000, stdio: 'ignore' });
    }
    IosGate.device = undefined;
  }
  rmSync(IosGate.project, { recursive: true, force: true });
};
process.once('SIGINT', () => { cleanup(); process.exit(130); });
process.once('SIGTERM', () => { cleanup(); process.exit(143); });

function writeFixture() {
  if (!IosGate.chosenDevice) mkdirSync(IosGate.deviceSet);
  write('client/manifest.json', JSON.stringify({ name: 'NativeIOSGate', start_url: '/', tac: { application: { id: 'dev.tachyon.ios-gate', version: '0.0.1', build: 4 }, browserApis: ['clipboard'], styles: ['/shared/ios-gate.css'], names: { check: false }, constants: { check: false }, visibility: { check: false }, duplication: { check: false }, cohesion: { check: false } } }));
  write('client/models/IosEvent.js', '/**\n * @typedef {Object} IosEvent\n * @property {string} text\n */\nexport {}\n');
  write('client/models/IosEvent.swift', 'struct IosEvent {\n    let text: String\n}\n');
  write('client/shared/ios-gate.js', `export const marker = 'Shared module ready';
export const bridgedApis = () => globalThis.__tachyonBrowserApiBridge?.join(',');
const hostCall = globalThis.__tachyonNativeHostCall;
globalThis.__tachyonNativeHostCall = (capability, payload) => {
  const request = JSON.parse(payload);
  if (capability === 'companion.invoke' && request.op === 'init' && request.service && request.service !== 'iosTally' && !globalThis.iosGateService) { globalThis.iosGateOwner = request.owner; globalThis.iosGateService = request.service; }
  return hostCall(capability, payload);
};\n`);
  write('client/shared/relaunch-storage.js', RelaunchStorageProbe.pageModule());
  write('client/shared/ios-gate.css', 'body { --ios-gate: ready; font-family: sans-serif; }\n');
  for (const [prefix, label, value] of [['', 'root', 7], ['items/_id/', 'second', 9]]) {
    const service = 'Ios' + label[0].toUpperCase() + label.slice(1) + 'Service';
    const instance = 'ios' + label[0].toUpperCase() + label.slice(1);
    write('client/pages/' + prefix + 'tac.html', `<ios-${label} />`);
    const tally = prefix ? '' : '<p id="tally">{iosTally.count}</p><p id="tally-origin">{iosTally.origin}</p><button id="tally-double" on:click="iosTally.double()">Double in Rust</button>';
    write('client/components/ios/' + label + '/tac.html', `<main><h1 id="heading">${label} route</h1><p id="marker">{${instance}.marker}</p>${tally}
<p id="count">{${instance}.count}</p><p id="signal">{${instance}.signal}</p>
<button id="update" on:click="${instance}.count = ${value}">Update native</button>
<input id="name" :value="${instance}.name" on:input="${instance}.name = $event.target.value" aria-label="Name">
<a id="next" href="${prefix ? '/?returned=1' : '/items/7/'}">Next route</a></main>`);
    write('client/services/' + service + '.js', String.raw`import { marker, bridgedApis } from '/shared/ios-gate.js';
import { RelaunchStorage } from '/shared/relaunch-storage.js';
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
  async linkedRust() {
    await this.until(() => document.querySelector('#tally')?.textContent === '21');
    document.querySelector('#tally-double').click();
    await this.until(() => document.querySelector('#tally')?.textContent === '42');
    return document.querySelector('#tally-origin').textContent + ':' + document.querySelector('#tally').textContent;
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
      const storage = this.pageLabel === 'root' && !returned ? await RelaunchStorage.check() : 'not checked';
      const relaunched = this.pageLabel === 'root' && !returned && !storage.startsWith('${RelaunchStorageProbe.WRITTEN}');
      if (this.pageLabel === 'root' && this.saved !== (returned || relaunched ? 'kept' : '')) throw new Error('LocalState after a native method and a page load: ' + JSON.stringify({ returned, relaunched, saved: this.saved, stored: Object.entries(localStorage), path: location.href }));
      if (this.pageLabel === 'root' && !returned && await this.remember() !== 'kept') throw new Error('Native LocalState method');
      if (document.querySelector('#marker').textContent !== 'Shared module ready') throw new Error('Shared JS');
      const rustTally = this.pageLabel === 'root' && !returned ? await this.linkedRust() : 'not checked';
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
      if (this.pageLabel === 'root' && !returned) {
        if (bridgedApis() !== 'clipboard,share,notifications,geolocation') throw new Error('Native browser API bridge list');
        const written = JSON.parse(await __tachyonNativeHostCall('browserApi.clipboard.writeText', JSON.stringify({ text: 'tachyon ios bridge' })));
        const read = JSON.parse(await __tachyonNativeHostCall('browserApi.clipboard.readText', '{}'));
        const share = JSON.parse(await __tachyonNativeHostCall('browserApi.share.open', JSON.stringify({ text: 'not granted' })));
        if (written.value !== null || read.value !== 'tachyon ios bridge' || !String(share.error).includes('not granted')) throw new Error('Native clipboard bridge: ' + JSON.stringify({ written, read, share }));
      }
      await this.announce();
      await this.until(() => document.querySelector('#signal')?.textContent === 'received');
      this.phase = returned ? 'returned' : 'initial';
      this.proof = JSON.stringify({ route: this.pageLabel, count: this.count,
        doubled: await this.doubled(), nativePid: await this.processId(),
        sharedAssets: true, renderedInteraction: true, focusPreserved: true,
        routeRejected: true, duplicateRouteRejected: true, unicodeBoundaryPassed: true, clipboardBridge: true,
        publishReceived: true, saved: this.saved, storage, rustTally });
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
  write('client/services/IosTallyService.rs', `#[Service]
#[derive(Default)]
struct IosTallyService {
    count: i64,
    origin: String,
}

impl IosTallyService {
    #[OnStart]
    fn initialize(&mut self) {
        self.count = 21;
        self.origin = String::from(if cfg!(target_os = "ios") { "ios" } else { "elsewhere" });
        let home = std::env::var("HOME").unwrap_or_default();
        let proof = format!("{{\\"pid\\":{},\\"origin\\":\\"{}\\"}}", std::process::id(), self.origin);
        let _ = std::fs::write(format!("{home}/Documents/rust-start.json"), proof);
    }

    pub fn double(&mut self) {
        self.count *= 2;
    }
}
`);
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
  console.log('Building actual iOS host with two Swift services and a linked Rust service...');
  run(binary, ['build', IosGate.project, '--target', 'ios'], 240000);
  const output = path.join(IosGate.project, 'dist/ios');
  const bundle = path.join(output, 'NativeIOSGate.app');
  const manifest = JSON.parse(readFileSync(path.join(output, 'tachyon.host.json'), 'utf8'));
  assert.equal(manifest.schemaVersion, 3);
  assert.deepEqual(manifest.companions, [
    { service: 'iosRoot', language: 'Swift' }, { service: 'iosSecond', language: 'Swift' },
    { service: 'iosTally', language: 'Rust' },
  ]);
  assert.deepEqual(readFileSync(path.join(output, 'web/index.html')),
    readFileSync(path.join(bundle, 'WebBundle/index.html')));
  const plist = path.join(bundle, 'Info.plist');
  assert.equal(run('/usr/bin/plutil', ['-extract', 'CFBundleShortVersionString', 'raw', plist]), '0.0.1');
  assert.equal(run('/usr/bin/plutil', ['-extract', 'CFBundleVersion', 'raw', plist]), '4');
  return bundle;
}

function opaqueIcon(edge) {
  const chunk = (type, data) => {
    const body = Buffer.concat([Buffer.from(type), data]);
    const framed = Buffer.alloc(body.length + 8);
    framed.writeUInt32BE(data.length, 0);
    body.copy(framed, 4);
    framed.writeUInt32BE(crc32(body), body.length + 4);
    return framed;
  };
  const header = Buffer.alloc(13);
  header.writeUInt32BE(edge, 0);
  header.writeUInt32BE(edge, 4);
  header[8] = 8;
  header[9] = 2;
  const row = Buffer.concat([Buffer.from([0]), Buffer.alloc(edge * 3, 0x33)]);
  return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk('IHDR', header),
    chunk('IDAT', deflateSync(Buffer.concat(Array(edge).fill(row)))), chunk('IEND', Buffer.alloc(0))]);
}

function checkSignedApp(output, platform) {
  const bundle = path.join(output, 'NativeIOSGate.app');
  run('/usr/bin/codesign', ['--verify', '--strict', bundle]);
  const plist = path.join(bundle, 'Info.plist');
  assert.equal(run('/usr/bin/plutil', ['-extract', 'CFBundleSupportedPlatforms.0', 'raw', plist]), platform);
  for (const file of ['embedded.mobileprovision', 'PrivacyInfo.xcprivacy']) {
    assert.ok(existsSync(path.join(bundle, file)), file + ' must be in the signed app');
  }
  const listing = run('/usr/bin/unzip', ['-l', path.join(output, 'NativeIOSGate.ipa')]);
  assert.match(listing, /Payload\/NativeIOSGate\.app\/Info\.plist/);
  return bundle;
}

function buildSignedApps(binary) {
  const output = path.join(IosGate.project, 'dist/ios');
  if (!process.env.TAC_APPLE_TEAM_ID || !process.env.TAC_IOS_PROVISIONING_PROFILE) {
    console.log('SKIP: device build; set TAC_APPLE_TEAM_ID and TAC_IOS_PROVISIONING_PROFILE (a development or ad hoc profile for ' + IosGate.appId + ')');
  } else {
    console.log('Building and signing the iOS app for a device...');
    run(binary, ['bundle', IosGate.project, '--target', 'ios', '--device'], 300000);
    const bundle = checkSignedApp(output, 'iPhoneOS');
    if (process.env.TAC_IOS_DEVICE) {
      run('/usr/bin/xcrun', ['devicectl', 'device', 'install', 'app', '--device', process.env.TAC_IOS_DEVICE, bundle], 300000);
      run('/usr/bin/xcrun', ['devicectl', 'device', 'process', 'launch', '--device', process.env.TAC_IOS_DEVICE, IosGate.appId], 120000);
    } else {
      console.log('SKIP: device install; set TAC_IOS_DEVICE to a paired device identifier');
    }
  }
  const storeProfile = process.env.TAC_IOS_GATE_APP_STORE_PROFILE;
  if (!process.env.TAC_APPLE_TEAM_ID || !storeProfile) {
    console.log('SKIP: App Store build; set TAC_APPLE_TEAM_ID and TAC_IOS_GATE_APP_STORE_PROFILE (an App Store profile for ' + IosGate.appId + ')');
    return;
  }
  console.log('Building the App Store .ipa...');
  write('client/shared/ios-gate-icon.png', opaqueIcon(64));
  const manifestPath = path.join(IosGate.project, 'client/manifest.json');
  const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
  manifest.icons = [{ src: '/shared/ios-gate-icon.png', sizes: '64x64', type: 'image/png' }];
  writeFileSync(manifestPath, JSON.stringify(manifest));
  const result = spawnSync(binary, ['bundle', IosGate.project, '--target', 'ios', '--release'], {
    cwd: IosGate.repo, encoding: 'utf8', timeout: 300000, maxBuffer: 2 * 1024 * 1024,
    env: { ...process.env, TAC_IOS_PROVISIONING_PROFILE: storeProfile },
  });
  assert.equal(result.status, 0, 'ty bundle --release: ' + (result.error || result.stderr || result.stdout));
  checkSignedApp(output, 'iPhoneOS');
  assert.ok(existsSync(path.join(output, 'NativeIOSGate.app/Assets.car')), 'the App Store app must carry its asset catalog');
}

function chosenVersion(version) {
  const wanted = IosGate.runtimeVersion;
  return !wanted || version === wanted || version.startsWith(wanted + '.');
}

function useBootedSimulator(runtimes) {
  const devices = JSON.parse(simctl('list', 'devices', '--json')).devices;
  const [identifier, found] = Object.entries(devices)
    .map(([key, list]) => [key, list.find(item => item.udid === IosGate.chosenDevice)])
    .find(([, item]) => item) ?? [];
  assert.ok(found, 'TAC_IOS_DEVICE ' + IosGate.chosenDevice + ' is not in the device set ' + IosGate.deviceSet);
  assert.equal(found.state, 'Booted', 'TAC_IOS_DEVICE ' + IosGate.chosenDevice + ' must already be booted');
  const runtime = runtimes.find(item => item.identifier === identifier);
  assert.ok(runtime && chosenVersion(runtime.version), 'TAC_IOS_DEVICE ' + IosGate.chosenDevice + ' runs ' + identifier + ', not iOS ' + IosGate.runtimeVersion);
  const phone = (runtime.supportedDeviceTypes ?? []).find(item => item.identifier === found.deviceTypeIdentifier) ?? found;
  IosGate.device = IosGate.chosenDevice;
  console.log('Using booted ' + phone.name + ' with iOS ' + runtime.version + '...');
  return { runtime, phone };
}

function bootSimulator() {
  const runtimes = JSON.parse(simctl('list', 'runtimes', '--json')).runtimes;
  if (IosGate.chosenDevice) return useBootedSimulator(runtimes);
  const runtime = runtimes.filter(item => item.isAvailable && item.identifier.includes('.iOS-') && chosenVersion(item.version))
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
  run('/usr/bin/xcrun', ['simctl', '--set', IosGate.deviceSet, 'bootstatus', IosGate.device, '-b'], 600000);
  return { runtime, phone };
}

function launchApp(bundle) {
  slowSimctl('install', IosGate.device, bundle);
  return startApp();
}

function startApp() {
  const launch = slowSimctl('launch', IosGate.device, IosGate.appId);
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
      'duplicateRouteRejected', 'unicodeBoundaryPassed', 'clipboardBridge', 'publishReceived']) {
      assert.equal(proof[key], true, name + ': ' + key);
    }
    assert.equal(proof.saved, name.startsWith('root') ? 'kept' : '', name + ': LocalState after a native method');
    assert.equal(proof.rustTally, name === 'root-initial.json' ? 'ios:42' : 'not checked', name + ': the linked Rust service');
    return { file: name, ...native, proof };
  });
  return proofs;
}

function processRunning(pid) {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

async function quitApp(pid) {
  simctl('terminate', IosGate.device, IosGate.appId);
  const deadline = Date.now() + 30000;
  while (processRunning(pid) && Date.now() < deadline) await pause(100);
  assert.ok(!processRunning(pid), 'the app process ' + pid + ' must end before it is launched again');
}

function keptStorage(result) {
  const parts = {};
  for (const part of result.split(',')) {
    const [name, state] = part.split('=');
    parts[name] = state;
  }
  assert.equal(parts.local, 'kept', 'the localStorage value was not kept after the app quit, got ' + result);
  assert.equal(parts.store, 'kept', 'the IndexedDB row was not kept after the app quit, got ' + result);
  assert.equal(parts.cache, 'kept', 'the Cache API entry was not kept after the app quit, got ' + result);
  return { result, cache: parts.cache };
}

function storageResult(proofs) {
  return proofs.find(proof => proof.file === IosGate.storageProof).proof.storage;
}

async function relaunchApp(documents, required, firstPid) {
  await quitApp(firstPid);
  for (const name of required) rmSync(path.join(documents, name), { force: true });
  const pid = startApp();
  assert.notEqual(pid, firstPid, 'the relaunch must start a new app process');
  await waitForProofFiles();
  return { pid, proofs: collectProofs(documents, required, pid) };
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
    const rust = JSON.parse(readFileSync(path.join(documents, 'rust-start.json'), 'utf8'));
    assert.deepEqual(rust, { pid, origin: 'ios' }, 'the Rust service must run natively in the app process');
    assert.equal(simctl('pbpaste', IosGate.device), 'tachyon ios bridge', 'the page clipboard write must reach UIPasteboard');
    const written = storageResult(proofs);
    assert.equal(written, RelaunchStorageProbe.WRITTEN, 'the first launch must write web storage, got ' + written);
    const screenshot = path.join(IosGate.project, 'ios-returned-root.png');
    simctl('io', IosGate.device, 'screenshot', screenshot);
    copyFileSync(screenshot, path.join(IosGate.evidence, 'ios-returned-root.png'));
    const relaunch = await relaunchApp(documents, required, pid);
    const storage = keptStorage(storageResult(relaunch.proofs));
    console.log('iOS relaunch storage passed: launch ' + pid + ' ' + written + ', relaunch ' + relaunch.pid +
      ' kept local and store; cache=' + storage.cache);
    writeFileSync(path.join(IosGate.evidence, 'report.json'), JSON.stringify({
      runtime: runtime.version, deviceType: phone.name, binary, pid, proofs, relaunch, storage,
    }, null, 2) + '\n');
    buildSignedApps(binary);
    console.log('PASS: real iOS WKWebView render/input/focus, Swift ABI/OS/publish, linked Rust service, clipboard bridge, shared assets, dynamic routes, isolation, ' +
      'LocalState kept after a native method and a page load, and relaunch storage (local=kept, store=kept, cache=' + storage.cache + ')');
  } finally {
    cleanup();
  }
  run(process.execPath, [path.join(IosGate.repo, 'scripts/ios/window-settings-test.mjs')], 1800000);
  console.log('PASS: iOS window settings on an iPhone and an iPad simulator');
}

await main();
