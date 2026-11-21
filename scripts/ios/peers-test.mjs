#!/usr/bin/env node
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { PeersNativeProbe } from '../native/peers-native-probe.mjs';
import { SharedSimulator } from './shared-simulator.mjs';

class IosPeersGate {
  static repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
  static root = mkdtempSync(path.join(tmpdir(), 'tachyon-ios-peers-'));
  static deviceSet = SharedSimulator.deviceSet(path.join(IosPeersGate.root, 'simulator-devices'));
  static installMilliseconds = 900000;
  static appId = 'dev.tachyon.peers-gate';
  static appName = 'PeersGate';
  static reader = path.join(IosPeersGate.root, 'screenshot-text');
  static reason = 'Connects to your other devices';
  static waitRounds = 60;
  static devices = [];
  static usedDevices = [];

  static run(program, args, timeout = 240000) {
    const result = spawnSync(program, args, { cwd: IosPeersGate.repo, encoding: 'utf8', timeout, maxBuffer: 16 * 1024 * 1024 });
    assert.equal(result.status, 0, `${program} ${args.join(' ')}: ${result.error || result.stderr || result.stdout}`);
    return result.stdout.trim();
  }

  static simctl(...args) {
    return IosPeersGate.run('/usr/bin/xcrun', ['simctl', '--set', IosPeersGate.deviceSet, ...args]);
  }

  static slowSimctl(...args) {
    return IosPeersGate.run('/usr/bin/xcrun', ['simctl', '--set', IosPeersGate.deviceSet, ...args], IosPeersGate.installMilliseconds);
  }

  static pause(milliseconds) {
    return new Promise(resolve => setTimeout(resolve, milliseconds));
  }

  static binary() {
    if (process.env.TAC_BIN) return process.env.TAC_BIN;
    IosPeersGate.run('cargo', ['build', '--locked', '--bin', 'ty'], 1200000);
    return path.join(JSON.parse(IosPeersGate.run('cargo', ['metadata', '--format-version', '1', '--no-deps'])).target_directory, 'debug/ty');
  }

  static build(binary, port) {
    const project = path.join(IosPeersGate.root, 'project');
    const settings = { id: IosPeersGate.appId, name: IosPeersGate.appName, permissions: { peers: IosPeersGate.reason }, ...PeersNativeProbe.relayed(port) };
    for (const [name, text] of Object.entries(PeersNativeProbe.files(settings))) {
      const file = path.join(project, name);
      mkdirSync(path.dirname(file), { recursive: true });
      writeFileSync(file, text);
    }
    IosPeersGate.run(binary, ['build', project, '--target', 'ios'], 600000);
    IosPeersGate.run('/usr/bin/xcrun', ['swiftc', '-parse-as-library', '-O', path.join(IosPeersGate.repo, 'scripts/ios/screenshot-text.swift'), '-o', IosPeersGate.reader], 600000);
    const bundle = path.join(project, 'dist/ios', `${IosPeersGate.appName}.app`);
    const plist = readFileSync(path.join(bundle, 'Info.plist'), 'utf8');
    assert.ok(plist.includes(`<key>NSLocalNetworkUsageDescription</key><string>${IosPeersGate.reason}</string>`), 'an iOS app that grants peers carries the local network reason in its Info.plist');
    return bundle;
  }

  static boot() {
    const { simctl, run, deviceSet } = IosPeersGate;
    if (SharedSimulator.udid('iPhone')) {
      const { device } = SharedSimulator.use(deviceSet, 'iPhone');
      IosPeersGate.usedDevices.push(device);
      return device;
    }
    const runtime = JSON.parse(simctl('list', 'runtimes', '--json')).runtimes
      .filter(item => item.isAvailable && item.identifier.includes('.iOS-') && SharedSimulator.wantedRuntime(item))
      .sort((a, b) => b.version.localeCompare(a.version, undefined, { numeric: true }))[0];
    assert.ok(runtime, 'an installed iOS simulator runtime is required');
    const type = runtime.supportedDeviceTypes.find(item => item.productFamily === 'iPhone' && !item.name.includes('SE'));
    assert.ok(type, `the iOS ${runtime.version} runtime needs an iPhone device type`);
    const device = simctl('create', `TachyonPeers-${randomUUID()}`, type.identifier, runtime.identifier);
    IosPeersGate.devices.push(device);
    simctl('boot', device);
    run('/usr/bin/xcrun', ['simctl', '--set', deviceSet, 'bootstatus', device, '-b'], 300000);
    return device;
  }

  static squeeze(text) {
    return text.toLowerCase().replace(/\s+/g, '');
  }

  static async shown(device) {
    const { simctl, run, reader, root, pause, squeeze, waitRounds } = IosPeersGate;
    let text = '';
    for (let round = 0; round < waitRounds; round++) {
      await pause(1000);
      const file = path.join(root, 'peers.png');
      simctl('io', device, 'screenshot', file);
      text = JSON.parse(run(reader, [file])).sort((a, b) => a.y - b.y).map(({ text: line }) => line).join(' ');
      if (squeeze(text).includes(squeeze(PeersNativeProbe.PREFIX)) && !squeeze(text).includes(squeeze(PeersNativeProbe.CHECKING))) return text;
    }
    return text;
  }

  static cleanup(turn) {
    turn?.kill('SIGTERM');
    for (const device of IosPeersGate.devices) SharedSimulator.remove(IosPeersGate.deviceSet, device);
    for (const device of IosPeersGate.usedDevices) SharedSimulator.release(IosPeersGate.deviceSet, device, [IosPeersGate.appId]);
    rmSync(IosPeersGate.root, { recursive: true, force: true });
  }

  static async main() {
    const port = await PeersNativeProbe.freePort();
    const turn = PeersNativeProbe.startTurn(port);
    try {
      mkdirSync(IosPeersGate.deviceSet, { recursive: true });
      const bundle = IosPeersGate.build(IosPeersGate.binary(), port);
      const device = IosPeersGate.boot();
      spawnSync('/usr/bin/xcrun', ['simctl', '--set', IosPeersGate.deviceSet, 'uninstall', device, IosPeersGate.appId], { stdio: 'ignore', timeout: 60000 });
      IosPeersGate.slowSimctl('install', device, bundle);
      IosPeersGate.slowSimctl('launch', device, IosPeersGate.appId);
      const shown = await IosPeersGate.shown(device);
      spawnSync('/usr/bin/xcrun', ['simctl', '--set', IosPeersGate.deviceSet, 'terminate', device, IosPeersGate.appId], { stdio: 'ignore', timeout: 30000 });
      assert.ok(IosPeersGate.squeeze(shown).includes(IosPeersGate.squeeze(PeersNativeProbe.PREFIX + PeersNativeProbe.PASSED)), `two Peers connections in the iOS WKWebView must connect through the local TURN relay and carry ${PeersNativeProbe.TOTAL} bytes, the screen shows: ${shown}`);
      console.log(`PASS: in the iOS app on the simulator (tachyon-app://bundle), Peers connected two connections through a local coturn relay with relayOnly and delivered ${PeersNativeProbe.TOTAL} bytes; Info.plist carries NSLocalNetworkUsageDescription`);
    } finally {
      IosPeersGate.cleanup(turn);
    }
  }
}

process.once('SIGINT', () => { IosPeersGate.cleanup(); process.exit(130); });
await IosPeersGate.main();
