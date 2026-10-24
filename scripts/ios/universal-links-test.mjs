#!/usr/bin/env node
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

assert.equal(process.platform, 'darwin', 'the iOS universal links check needs macOS and Xcode');

class IosUniversalLinksGate {
  static repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
  static project = mkdtempSync(path.join(tmpdir(), 'tachyon-ios-universal-links-'));
  static deviceSet = path.join(IosUniversalLinksGate.project, 'simulator-devices');
  static host = process.env.TAC_LINKS_HOST ?? 'd31ma.github.io';
  static team = process.env.TAC_APPLE_TEAM_ID;
  static appId = 'dev.tachyon.link-gate';
  static devices = [];

  static run(program, args, timeout = 240000) {
    const result = spawnSync(program, args, {
      cwd: IosUniversalLinksGate.repo, encoding: 'utf8', timeout, maxBuffer: 16 * 1024 * 1024,
    });
    assert.equal(result.status, 0, program + ' ' + args.join(' ') + ': ' +
      (result.error || result.stderr || result.stdout));
    return result.stdout.trim();
  }

  static simctl(...args) {
    return IosUniversalLinksGate.run('/usr/bin/xcrun', ['simctl', '--set', IosUniversalLinksGate.deviceSet, ...args]);
  }

  static write(relative, source) {
    const file = path.join(IosUniversalLinksGate.project, relative);
    mkdirSync(path.dirname(file), { recursive: true });
    writeFileSync(file, source);
  }

  static pause(milliseconds) {
    return new Promise(resolve => setTimeout(resolve, milliseconds));
  }

  static writeFixture() {
    const { write, appId, host } = IosUniversalLinksGate;
    const checks = { names: { check: false }, constants: { check: false }, visibility: { check: false }, duplication: { check: false }, cohesion: { check: false } };
    const application = { id: appId, version: '0.0.1', build: 1, links: { schemes: ['linkgate'], hosts: [host] } };
    write('client/manifest.json', JSON.stringify({ name: 'LinkGate', start_url: '/', tac: { origin: `https://${host}`, application, ...checks } }));
    for (const [prefix, page] of [['', 'home'], ['orders/', 'orders']]) {
      write('client/pages/' + prefix + 'tac.html', `<link-${page} />`);
      write('client/components/link/' + page + '/tac.html', `<main><h1>${page}</h1><p id="path">{linkProbe.path}</p></main>`);
    }
    write('client/services/LinkProbeService.js', `@Service()
export default class LinkProbeService {
  path = '';
  @OnStart()
  initialize() {
    setTimeout(() => { void this.report(); }, 50);
  }
  async report() {
    this.path = location.pathname + location.search;
    await this.record();
  }
}
`);
    write('client/services/LinkProbeService.swift', `import Foundation
@Service
final class LinkProbeService {
    var path: String = ""
    func record() -> Int {
        let directory = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
        let file = directory.appendingPathComponent("link-pages.txt")
        let pages = (try? String(contentsOf: file, encoding: .utf8)) ?? ""
        try? (pages + path + "\\n").write(to: file, atomically: true, encoding: .utf8)
        return 1
    }
}
`);
  }

  static binary() {
    if (process.env.TAC_BIN) return process.env.TAC_BIN;
    IosUniversalLinksGate.run('cargo', ['build', '--locked', '--bin', 'ty']);
    const metadata = JSON.parse(IosUniversalLinksGate.run('cargo', ['metadata', '--format-version', '1', '--no-deps']));
    return path.join(metadata.target_directory, 'debug/ty');
  }

  static build() {
    const { run, project, host, team, appId } = IosUniversalLinksGate;
    const result = spawnSync(IosUniversalLinksGate.binary(), ['build', project, '--target', 'ios'], {
      cwd: IosUniversalLinksGate.repo, encoding: 'utf8', timeout: 300000, maxBuffer: 4 * 1024 * 1024,
    });
    assert.equal(result.status, 0, 'ty build --target ios: ' + (result.error || result.stderr || result.stdout));
    const bundle = path.join(project, 'dist/ios/LinkGate.app');
    const executable = readFileSync(path.join(bundle, 'LinkGate'));
    for (const text of [`<string>applinks:${host}</string>`, `<string>${team}.${appId}</string>`]) {
      assert.ok(executable.includes(text), `the app's __TEXT,__entitlements section holds ${text}`);
    }
    const signed = run('/usr/bin/codesign', ['-d', '--entitlements', ':-', bundle]);
    assert.ok(!signed.includes('applinks:'), 'the ad hoc signature carries no restricted entitlements: ' + signed);
    console.log(`OK: the simulator app holds applinks:${host} and ${team}.${appId} in its executable, and its ad hoc signature has no restricted entitlements`);
    return bundle;
  }

  static async checkServedFile() {
    const { host, team, appId } = IosUniversalLinksGate;
    const wanted = `${team}.${appId}`;
    for (const address of [`https://${host}/.well-known/apple-app-site-association`, `https://app-site-association.cdn-apple.com/a/v1/${host}`]) {
      const response = await fetch(address, { redirect: 'manual' });
      const text = response.status === 200 ? await response.text() : '';
      const listed = text && JSON.parse(text).applinks?.details?.some(({ appIDs, appID }) => (appIDs ?? [appID]).includes(wanted));
      assert.ok(listed, `${address} answered ${response.status} without ${wanted}; publish the files from scripts/native/verified-links-files.mjs first (Apple's CDN can take a while to pick up a change): ${text}`);
      console.log(`OK: ${address} lists ${wanted}`);
    }
  }

  static runtime() {
    const runtimes = JSON.parse(IosUniversalLinksGate.simctl('list', 'runtimes', '--json')).runtimes;
    const runtime = runtimes.filter(item => item.isAvailable && item.identifier.includes('.iOS-'))
      .sort((a, b) => b.version.localeCompare(a.version, undefined, { numeric: true }))[0];
    assert.ok(runtime, 'an installed iOS simulator runtime is required');
    return runtime;
  }

  static boot(runtime) {
    const { simctl, run, deviceSet, devices } = IosUniversalLinksGate;
    const type = runtime.supportedDeviceTypes.find(item => item.productFamily === 'iPhone');
    assert.ok(type, 'the runtime needs an iPhone device type');
    const device = simctl('create', 'TachyonLinks-' + randomUUID(), type.identifier, runtime.identifier);
    devices.push(device);
    console.log('Booting ' + type.name + ' with iOS ' + runtime.version + '...');
    simctl('boot', device);
    run('/usr/bin/xcrun', ['simctl', '--set', deviceSet, 'bootstatus', device, '-b'], 300000);
    return { device, name: type.name };
  }

  static pages(device) {
    const container = IosUniversalLinksGate.simctl('get_app_container', device, IosUniversalLinksGate.appId, 'data');
    const file = path.join(container, 'Documents/link-pages.txt');
    if (!existsSync(file)) return [];
    return readFileSync(file, 'utf8').split('\n').filter(Boolean);
  }

  static associationLog(device) {
    const result = spawnSync('/usr/bin/xcrun', ['simctl', '--set', IosUniversalLinksGate.deviceSet, 'spawn', device, 'log', 'show',
      '--last', '5m', '--style', 'compact', '--predicate', 'process == "swcd" OR eventMessage CONTAINS[c] "universal link"'],
    { encoding: 'utf8', timeout: 120000, maxBuffer: 16 * 1024 * 1024 });
    return (result.stdout || '').split('\n').slice(-40).join('\n');
  }

  static async waitForPage(device, page, message) {
    for (let attempt = 0; attempt < 120; attempt++) {
      if (IosUniversalLinksGate.pages(device).at(-1) === page) return;
      await IosUniversalLinksGate.pause(250);
    }
    assert.fail(message + ': ' + JSON.stringify(IosUniversalLinksGate.pages(device)) + '\nswcd log:\n' + IosUniversalLinksGate.associationLog(device));
  }

  static async check(runtime, bundle) {
    const { simctl, boot, waitForPage, appId, host, pause } = IosUniversalLinksGate;
    const { device, name } = boot(runtime);
    simctl('install', device, bundle);
    simctl('launch', device, appId);
    await waitForPage(device, '/index.html', name + ' opens the entry page');
    simctl('terminate', device, appId);
    await pause(5000);
    simctl('openurl', device, `https://${host}/orders?x=1`);
    await waitForPage(device, '/orders?x=1', name + `: a cold start from https://${host}/orders?x=1 opens its route in the app`);
    console.log(`OK: ${name}: https://${host}/orders?x=1 started the app at /orders?x=1`);
    simctl('openurl', device, `https://${host}/orders?x=2`);
    await waitForPage(device, '/orders?x=2', name + `: https://${host}/orders?x=2 reaches the running app`);
    console.log(`OK: ${name}: https://${host}/orders?x=2 reached the running app`);
    simctl('shutdown', device);
  }

  static cleanup() {
    for (const device of IosUniversalLinksGate.devices) {
      for (const args of [['shutdown', device], ['delete', device]]) {
        spawnSync('/usr/bin/xcrun', ['simctl', '--set', IosUniversalLinksGate.deviceSet, ...args], { timeout: 60000, stdio: 'ignore' });
      }
    }
    rmSync(IosUniversalLinksGate.project, { recursive: true, force: true });
  }

  static async main() {
    assert.match(IosUniversalLinksGate.team ?? '', /^[A-Z0-9]{10}$/, 'TAC_APPLE_TEAM_ID is required');
    try {
      mkdirSync(IosUniversalLinksGate.deviceSet);
      IosUniversalLinksGate.writeFixture();
      await IosUniversalLinksGate.checkServedFile();
      const bundle = IosUniversalLinksGate.build();
      await IosUniversalLinksGate.check(IosUniversalLinksGate.runtime(), bundle);
      console.log(`PASS: iOS universal links: https://${IosUniversalLinksGate.host} links open the simulator app at their route on a cold start and while it runs`);
    } finally {
      IosUniversalLinksGate.cleanup();
    }
  }
}

process.once('SIGINT', () => { IosUniversalLinksGate.cleanup(); process.exit(130); });
process.once('SIGTERM', () => { IosUniversalLinksGate.cleanup(); process.exit(143); });
await IosUniversalLinksGate.main();
