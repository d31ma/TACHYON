#!/usr/bin/env node
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

assert.equal(process.platform, 'darwin', 'the iOS menus check needs macOS and Xcode');

class IosMenusGate {
  static repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
  static project = mkdtempSync(path.join(tmpdir(), 'tachyon-ios-menus-'));
  static deviceSet = path.join(IosMenusGate.project, 'simulator-devices');
  static keyboard = path.join(IosMenusGate.project, 'simulator-keyboard');
  static appId = 'dev.tachyon.menu-gate';
  static commandK = '0xE3,0x0E';
  static devices = [];
  static menus = {
    file: [{ id: 'status', label: 'Status', route: '/status', shortcut: { key: 'k', modifiers: ['command'] } }],
    help: [{ id: 'guide', label: 'Guide', route: '/guide' }],
  };

  static run(program, args, timeout = 240000) {
    const result = spawnSync(program, args, {
      cwd: IosMenusGate.repo, encoding: 'utf8', timeout, maxBuffer: 4 * 1024 * 1024,
    });
    assert.equal(result.status, 0, program + ' ' + args.join(' ') + ': ' +
      (result.error || result.stderr || result.stdout));
    return result.stdout.trim();
  }

  static simctl(...args) {
    return IosMenusGate.run('/usr/bin/xcrun', ['simctl', '--set', IosMenusGate.deviceSet, ...args]);
  }

  static write(relative, source) {
    const file = path.join(IosMenusGate.project, relative);
    mkdirSync(path.dirname(file), { recursive: true });
    writeFileSync(file, source);
  }

  static pause(milliseconds) {
    return new Promise(resolve => setTimeout(resolve, milliseconds));
  }

  static writeFixture() {
    const { write, menus, appId } = IosMenusGate;
    const checks = { names: { check: false }, constants: { check: false }, visibility: { check: false }, duplication: { check: false }, cohesion: { check: false } };
    write('client/manifest.json', JSON.stringify({ name: 'MenuGate', start_url: '/', tac: { application: { id: appId, version: '0.0.1', build: 1, menus }, ...checks } }));
    for (const [prefix, page] of [['', 'home'], ['status/', 'status'], ['guide/', 'guide']]) {
      write('client/pages/' + prefix + 'tac.html', `<menu-${page} />`);
      write('client/components/menu/' + page + '/tac.html', `<main><h1>${page}</h1><p id="path">{menuProbe.path}</p></main>`);
    }
    write('client/services/MenuProbeService.js', `@Service()
export default class MenuProbeService {
  path = '';
  @OnStart()
  initialize() {
    setTimeout(() => { void this.report(); }, 50);
  }
  async report() {
    this.path = location.pathname;
    await this.record();
  }
}
`);
    write('client/services/MenuProbeService.swift', `import Foundation
@Service
final class MenuProbeService {
    var path: String = ""
    func record() -> Int {
        let directory = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
        let file = directory.appendingPathComponent("menu-pages.txt")
        let pages = (try? String(contentsOf: file, encoding: .utf8)) ?? ""
        try? (pages + path + "\\n").write(to: file, atomically: true, encoding: .utf8)
        return 1
    }
}
`);
  }

  static binary() {
    if (process.env.TAC_BIN) return process.env.TAC_BIN;
    IosMenusGate.run('cargo', ['build', '--locked', '--bin', 'ty']);
    const metadata = JSON.parse(IosMenusGate.run('cargo', ['metadata', '--format-version', '1', '--no-deps']));
    return path.join(metadata.target_directory, 'debug/ty');
  }

  static build() {
    const { run, project } = IosMenusGate;
    const result = spawnSync(IosMenusGate.binary(), ['build', project, '--target', 'ios'], {
      cwd: IosMenusGate.repo, encoding: 'utf8', timeout: 300000, maxBuffer: 4 * 1024 * 1024,
    });
    assert.equal(result.status, 0, 'ty build --target ios: ' + (result.error || result.stderr || result.stdout));
    assert.match(result.stdout + result.stderr, /iPad menu bar \/ iPhone shortcuts/, 'the build notes how iOS adapts the menus');
    const source = readFileSync(path.join(project, 'dist/ios/project/TachyonHost.swift'), 'utf8');
    assert.ok(source.includes('IosMenuRoute(id: "status", label: "Status", group: "file", route: "/status", key: "k", modifiers: [.command])'));
    run('/usr/bin/xcrun', ['clang', '-fobjc-arc', '-framework', 'Foundation',
      path.join(IosMenusGate.repo, 'scripts/ios/simulator-keyboard.m'), '-o', IosMenusGate.keyboard]);
    return path.join(project, 'dist/ios/MenuGate.app');
  }

  static runtime() {
    const runtimes = JSON.parse(IosMenusGate.simctl('list', 'runtimes', '--json')).runtimes;
    const runtime = runtimes.filter(item => item.isAvailable && item.identifier.includes('.iOS-'))
      .sort((a, b) => b.version.localeCompare(a.version, undefined, { numeric: true }))[0];
    assert.ok(runtime, 'an installed iOS simulator runtime is required');
    return runtime;
  }

  static boot(runtime, family) {
    const { simctl, run, deviceSet, devices } = IosMenusGate;
    const type = runtime.supportedDeviceTypes.find(item => item.productFamily === family);
    assert.ok(type, 'the runtime needs an ' + family + ' device type');
    const device = simctl('create', 'TachyonMenus-' + randomUUID(), type.identifier, runtime.identifier);
    devices.push(device);
    console.log('Booting ' + type.name + ' with iOS ' + runtime.version + '...');
    simctl('boot', device);
    run('/usr/bin/xcrun', ['simctl', '--set', deviceSet, 'bootstatus', device, '-b'], 300000);
    return { device, name: type.name };
  }

  static pages(device) {
    const container = IosMenusGate.simctl('get_app_container', device, IosMenusGate.appId, 'data');
    const file = path.join(container, 'Documents/menu-pages.txt');
    if (!existsSync(file)) return [];
    return readFileSync(file, 'utf8').split('\n').filter(Boolean);
  }

  static async waitForPage(device, page, message) {
    for (let attempt = 0; attempt < 120; attempt++) {
      if (IosMenusGate.pages(device).at(-1) === page) return;
      await IosMenusGate.pause(250);
    }
    assert.fail(message + ': ' + JSON.stringify(IosMenusGate.pages(device)));
  }

  static async check(runtime, family, bundle) {
    const { simctl, run, boot, waitForPage, keyboard, deviceSet, commandK, appId } = IosMenusGate;
    const { device, name } = boot(runtime, family);
    simctl('install', device, bundle);
    simctl('launch', device, appId);
    await waitForPage(device, '/index.html', name + ' opens the entry page');
    await IosMenusGate.pause(1000);
    run(keyboard, [deviceSet, device, commandK]);
    await waitForPage(device, '/status', name + ': Command-K opens the Status menu item');
    const telemetry = path.join(simctl('get_app_container', device, appId, 'data'), 'Library/Logs/Tachyon', appId + '.jsonl');
    assert.match(readFileSync(telemetry, 'utf8'), /"event":"menu.opened"/, name + ' records the menu item');
    simctl('shutdown', device);
    console.log('PASS: ' + name + ': Command-K opened /status through the ' + (family === 'iPad' ? 'iPad menu bar' : 'iPhone key commands'));
  }

  static cleanup() {
    for (const device of IosMenusGate.devices) {
      for (const args of [['shutdown', device], ['delete', device]]) {
        spawnSync('/usr/bin/xcrun', ['simctl', '--set', IosMenusGate.deviceSet, ...args], { timeout: 60000, stdio: 'ignore' });
      }
    }
    rmSync(IosMenusGate.project, { recursive: true, force: true });
  }

  static async main() {
    try {
      mkdirSync(IosMenusGate.deviceSet);
      IosMenusGate.writeFixture();
      const bundle = IosMenusGate.build();
      const runtime = IosMenusGate.runtime();
      for (const family of ['iPhone', 'iPad']) {
        await IosMenusGate.check(runtime, family, bundle);
      }
      console.log('PASS: iOS menus: route items open their page from the iPad menu bar and from iPhone shortcuts');
    } finally {
      IosMenusGate.cleanup();
    }
  }
}

process.once('SIGINT', () => { IosMenusGate.cleanup(); process.exit(130); });
process.once('SIGTERM', () => { IosMenusGate.cleanup(); process.exit(143); });
await IosMenusGate.main();
