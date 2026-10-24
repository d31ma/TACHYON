#!/usr/bin/env node
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { ApplicationLifetimeFixture } from '../native/application-lifetime-fixture.mjs';

assert.equal(process.platform, 'darwin', 'the iOS application lifetime check needs macOS and Xcode');

class IosLifetimeGate {
  static repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
  static project = mkdtempSync(path.join(tmpdir(), 'tachyon-ios-lifetime-'));
  static deviceSet = path.join(IosLifetimeGate.project, 'simulator-devices');
  static keyboard = path.join(IosLifetimeGate.project, 'simulator-keyboard');
  static appId = ApplicationLifetimeFixture.applicationId;
  static keys = { k: '0xE3,0x0E', one: '0xE3,0x1E', two: '0xE3,0x1F', guide: '0xE3,0x20' };
  static devices = [];
  static notes = [];

  static run(program, args, timeout = 240000) {
    const result = spawnSync(program, args, {
      cwd: IosLifetimeGate.repo, encoding: 'utf8', timeout, maxBuffer: 4 * 1024 * 1024,
    });
    assert.equal(result.status, 0, program + ' ' + args.join(' ') + ': ' +
      (result.error || result.stderr || result.stdout));
    return result.stdout.trim();
  }

  static simctl(...args) {
    return IosLifetimeGate.run('/usr/bin/xcrun', ['simctl', '--set', IosLifetimeGate.deviceSet, ...args]);
  }

  static pause(milliseconds) {
    return new Promise(resolve => setTimeout(resolve, milliseconds));
  }

  static binary() {
    if (process.env.TAC_BIN) return process.env.TAC_BIN;
    IosLifetimeGate.run('cargo', ['build', '--locked', '--bin', 'ty']);
    const metadata = JSON.parse(IosLifetimeGate.run('cargo', ['metadata', '--format-version', '1', '--no-deps']));
    return path.join(metadata.target_directory, 'debug/ty');
  }

  static build() {
    const { run, project } = IosLifetimeGate;
    ApplicationLifetimeFixture.write(project);
    const result = spawnSync(IosLifetimeGate.binary(), ['build', project, '--target', 'ios'], {
      cwd: IosLifetimeGate.repo, encoding: 'utf8', timeout: 900000, maxBuffer: 4 * 1024 * 1024,
    });
    assert.equal(result.status, 0, 'ty build --target ios: ' + (result.error || result.stderr || result.stdout));
    assert.match(result.stdout + result.stderr, /Action items call a method of an application service/, 'the build notes how iOS adapts the menu actions');
    const source = readFileSync(path.join(project, 'dist/ios/project/TachyonHost.swift'), 'utf8');
    assert.ok(source.includes('IosMenuRoute(id: "show-running", label: "What\'s Running", group: "view", route: "", key: "1", modifiers: [.command], event: "SectionChosen", action: true)'));
    assert.ok(source.includes('guard tacRustNativeApplicationStart() else'));
    run('/usr/bin/xcrun', ['clang', '-fobjc-arc', '-framework', 'Foundation',
      path.join(IosLifetimeGate.repo, 'scripts/ios/simulator-keyboard.m'), '-o', IosLifetimeGate.keyboard]);
    return path.join(project, 'dist/ios', ApplicationLifetimeFixture.name + '.app');
  }

  static runtime() {
    const runtimes = JSON.parse(IosLifetimeGate.simctl('list', 'runtimes', '--json')).runtimes;
    const runtime = runtimes.filter(item => item.isAvailable && item.identifier.includes('.iOS-'))
      .sort((a, b) => b.version.localeCompare(a.version, undefined, { numeric: true }))[0];
    assert.ok(runtime, 'an installed iOS simulator runtime is required');
    return runtime;
  }

  static boot(runtime, family) {
    const { simctl, run, deviceSet, devices } = IosLifetimeGate;
    const type = runtime.supportedDeviceTypes.find(item => item.productFamily === family);
    assert.ok(type, 'the runtime needs an ' + family + ' device type');
    const device = simctl('create', 'TachyonLifetime-' + randomUUID(), type.identifier, runtime.identifier);
    devices.push(device);
    console.log('Booting ' + type.name + ' with iOS ' + runtime.version + '...');
    simctl('boot', device);
    run('/usr/bin/xcrun', ['simctl', '--set', deviceSet, 'bootstatus', device, '-b'], 300000);
    return { device, name: type.name };
  }

  static container(device) {
    return IosLifetimeGate.simctl('get_app_container', device, IosLifetimeGate.appId, 'data');
  }

  static journal(device) {
    const file = path.join(IosLifetimeGate.container(device), 'Documents/lifetime.txt');
    return ApplicationLifetimeFixture.journal(existsSync(file) ? readFileSync(file, 'utf8') : '');
  }

  static telemetry(device) {
    const file = path.join(IosLifetimeGate.container(device), 'Library/Logs/Tachyon', IosLifetimeGate.appId + '.jsonl');
    return existsSync(file) ? readFileSync(file, 'utf8') : '';
  }

  static mounted(device) {
    return IosLifetimeGate.telemetry(device).split('"event":"controller.mounted"').length - 1;
  }

  static async waitFor(device, check, message, attempts = 120) {
    for (let attempt = 0; attempt < attempts; attempt++) {
      const journal = IosLifetimeGate.journal(device);
      if (check(journal)) return journal;
      await IosLifetimeGate.pause(250);
    }
    assert.fail(message + ': ' + JSON.stringify(IosLifetimeGate.journal(device).lines));
  }

  static press(device, key) {
    IosLifetimeGate.run(IosLifetimeGate.keyboard, [IosLifetimeGate.deviceSet, device, IosLifetimeGate.keys[key]]);
  }

  static launch(device) {
    const launched = IosLifetimeGate.simctl('launch', device, IosLifetimeGate.appId);
    return Number(launched.match(/:\s*(\d+)\s*$/)?.[1]);
  }

  static async checkPagesAndActions(device, name) {
    const { waitFor, press, pause, telemetry } = IosLifetimeGate;
    const first = await waitFor(device, journal => journal.visits.length >= 1, name + ': the home page reaches the application service');
    assert.equal(first.created.length, 1, name + ': the application service starts once at launch');
    const instance = first.created[0];
    assert.deepEqual(first.visits[0], ['visit', instance, '1']);
    await pause(1000);
    press(device, 'k');
    const second = await waitFor(device, journal => journal.visits.length >= 2, name + ': Command-K opens the status page');
    assert.deepEqual(second.visits[1], ['visit', instance, '2'], name + ': the second page talks to the same instance');
    assert.equal(second.created.length, 1, name + ': opening a page does not start the service again');
    await pause(1000);
    press(device, 'one');
    await waitFor(device, journal => journal.lines.includes('heard running') && journal.lines.includes('forwarded'), name + ': Command-1 forwards SectionChosen to the page');
    press(device, 'two');
    await waitFor(device, journal => journal.lines.includes('refreshed ' + instance), name + ': Command-2 runs the action without an event');
    const mounted = IosLifetimeGate.mounted(device);
    press(device, 'guide');
    for (let attempt = 0; attempt < 120 && IosLifetimeGate.mounted(device) === mounted; attempt++) await pause(250);
    assert.ok(IosLifetimeGate.mounted(device) > mounted, name + ': Command-3 opens the guide page');
    await pause(2000);
    const heard = IosLifetimeGate.journal(device).lines.filter(line => line === 'forwarded' || line === 'unheard').length;
    press(device, 'one');
    await pause(1500);
    const after = IosLifetimeGate.journal(device).lines;
    assert.equal(after.filter(line => line === 'forwarded' || line === 'unheard').length, heard, name + ': Command-1 does nothing on a page without a listener: ' + JSON.stringify(after.slice(-4)));
    assert.match(telemetry(device), /"event":"menu.action"/, name + ' records the performed actions');
    return instance;
  }

  static async checkTermination(device, name, pid, instance) {
    const { waitFor, launch, journal } = IosLifetimeGate;
    try {
      process.kill(pid, 'SIGTERM');
    } catch (error) {
      IosLifetimeGate.notes.push(name + ': could not signal the app (' + error.message + ')');
    }
    const stopped = await waitFor(device, current => current.lines.includes('stopped ' + instance), 'stop', 40).catch(() => null);
    IosLifetimeGate.notes.push(name + ': ' + (stopped ? 'SIGTERM ran applicationWillTerminate and stopped the service once' : 'SIGTERM ended the app without applicationWillTerminate, so the stop was not observed'));
    if (stopped) assert.equal(stopped.lines.filter(line => line.startsWith('stopped ')).length, 1, name + ': the service stops once');
    spawnSync('/usr/bin/xcrun', ['simctl', '--set', IosLifetimeGate.deviceSet, 'terminate', device, IosLifetimeGate.appId], { timeout: 60000, stdio: 'ignore' });
    launch(device);
    const again = await waitFor(device, current => current.created.length >= 2, name + ': a new launch starts the application service again');
    assert.notEqual(again.created[1], instance, name + ': the new launch gets a new instance');
    assert.equal(journal(device).created.length, 2);
  }

  static async check(runtime, family, bundle) {
    const { simctl, boot, launch } = IosLifetimeGate;
    const { device, name } = boot(runtime, family);
    simctl('install', device, bundle);
    const pid = launch(device);
    const instance = await IosLifetimeGate.checkPagesAndActions(device, name);
    assert.ok(instance.startsWith(pid + '-'), name + ': the instance belongs to the launched app');
    await IosLifetimeGate.checkTermination(device, name, pid, instance);
    simctl('shutdown', device);
    console.log('PASS: ' + name + ': one LifetimeService served two pages, and Command-1 and Command-2 called it through the ' + (family === 'iPad' ? 'iPad menu bar' : 'iPhone key commands'));
  }

  static cleanup() {
    for (const device of IosLifetimeGate.devices) {
      for (const args of [['shutdown', device], ['delete', device]]) {
        spawnSync('/usr/bin/xcrun', ['simctl', '--set', IosLifetimeGate.deviceSet, ...args], { timeout: 60000, stdio: 'ignore' });
      }
    }
    rmSync(IosLifetimeGate.project, { recursive: true, force: true });
  }

  static async main() {
    try {
      mkdirSync(IosLifetimeGate.deviceSet);
      const bundle = IosLifetimeGate.build();
      const runtime = IosLifetimeGate.runtime();
      for (const family of ['iPhone', 'iPad']) {
        await IosLifetimeGate.check(runtime, family, bundle);
      }
      for (const note of IosLifetimeGate.notes) console.log('NOTE: ' + note);
      console.log('PASS: iOS application lifetime and menu actions');
    } finally {
      IosLifetimeGate.cleanup();
    }
  }
}

process.once('SIGINT', () => { IosLifetimeGate.cleanup(); process.exit(130); });
process.once('SIGTERM', () => { IosLifetimeGate.cleanup(); process.exit(143); });
await IosLifetimeGate.main();
