#!/usr/bin/env node
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { inflateSync } from 'node:zlib';
import { PageServicesFixture } from '../native/page-services-fixture.mjs';

assert.equal(process.platform, 'darwin', 'The iOS window settings check needs macOS and Xcode');

class IosWindowSettingsGate {
  static repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
  static root = mkdtempSync(path.join(tmpdir(), 'tachyon-ios-window-'));
  static deviceSet = path.join(IosWindowSettingsGate.root, 'simulator-devices');
  static appId = 'dev.tachyon.window-settings';
  static refusalId = 'dev.tachyon.window-refusal';
  static reader = path.join(IosWindowSettingsGate.root, 'screenshot-text');
  static evidence = process.env.TAC_IOS_WINDOW_EVIDENCE ?? path.join(IosWindowSettingsGate.repo, 'target/ios-window-settings-evidence');
  static devices = [];
  static styles = { default: 0, lightContent: 1, darkContent: 3 };
  static apps = {
    standard: {
      theme_color: '#0f6cbd', background_color: '#ffaa00', orientation: 'portrait',
      window: { controls: ['title'] },
    },
    integrated: {
      theme_color: '#ffffff', background_color: '#202020', orientation: 'portrait-primary',
      window: { titleBar: 'integrated' },
    },
    fullscreen: { display: 'fullscreen', orientation: 'landscape', window: {} },
  };

  static run(program, args, timeout = 240000) {
    const result = spawnSync(program, args, { cwd: IosWindowSettingsGate.repo, encoding: 'utf8', timeout, maxBuffer: 16 * 1024 * 1024 });
    assert.equal(result.status, 0, `${program} ${args.join(' ')}: ${result.error || result.stderr || result.stdout}`);
    return result.stdout.trim();
  }

  static simctl(...args) {
    return IosWindowSettingsGate.run('/usr/bin/xcrun', ['simctl', '--set', IosWindowSettingsGate.deviceSet, ...args]);
  }

  static pause(milliseconds) {
    return new Promise(resolve => setTimeout(resolve, milliseconds));
  }

  static write(project, relative, source) {
    const file = path.join(project, relative);
    mkdirSync(path.dirname(file), { recursive: true });
    writeFileSync(file, source);
  }

  static stage(name, settings) {
    const { write, appId } = IosWindowSettingsGate;
    const project = path.join(IosWindowSettingsGate.root, name);
    const checks = { check: false };
    const { window, ...members } = settings;
    write(project, 'client/manifest.json', JSON.stringify({
      name: 'WindowSettings', start_url: '/', ...members,
      tac: {
        application: { id: appId, version: '1.0.0', window },
        styles: ['/shared/window-probe.css'],
        names: checks, constants: checks, visibility: checks, duplication: checks, cohesion: checks,
      },
    }));
    write(project, 'client/shared/window-probe.css', 'html, body { margin: 0; background: transparent; font: 14px sans-serif; }\n');
    write(project, 'client/components/window/probe/tac.css', '#bar { height: var(--tac-titlebar-height, 0px); background: #ff00ff; }\nmain { padding: 8px; }\n');
    write(project, 'client/pages/tac.html', '<window-probe />\n');
    write(project, 'client/components/window/probe/tac.html', '<header id="bar"></header><main><p>{windowProbe.state}</p></main>');
    write(project, 'client/services/WindowProbeService.js', `@Service()
export default class WindowProbeService {
  state = 'waiting';
  proof = '';
  @OnStart()
  initialize() {
    setTimeout(() => { void this.probe(); }, 1500);
  }
  async probe() {
    try {
      const style = getComputedStyle(document.documentElement);
      const variable = name => style.getPropertyValue(name).trim();
      const page = {
        width: innerWidth, height: innerHeight,
        titleBarHeight: variable('--tac-titlebar-height'),
        leadingInset: variable('--tac-titlebar-leading-inset'),
        trailingInset: variable('--tac-titlebar-trailing-inset'),
        insetsAnswer: await __tachyonNativeHostCall('window.titleBarInsets', '{}').catch(error => 'refused: ' + error),
        barHeight: document.querySelector('#bar').getBoundingClientRect().height,
      };
      const native = JSON.parse(await this.facts());
      this.proof = JSON.stringify({ page, native });
      this.state = 'probed';
    } catch (error) {
      this.proof = JSON.stringify({ error: String(error) });
      this.state = 'failed';
    }
    await this.record();
  }
}
`);
    write(project, 'client/services/WindowProbeService.swift', `import Foundation
import UIKit
import WebKit

@Service
@NeedsAll
final class WindowProbeService {
    private let windowControls: WindowControls
    var state: String = "waiting"
    var proof: String = ""

    func facts() -> String {
        var facts: [String: Any] = [:]
        facts["grantTitle"] = windowControls.granted("title")
        facts["grantMinimize"] = windowControls.granted("minimize")
        do {
            try windowControls.setTitle("Quarterly report")
            facts["titleSet"] = "yes"
        } catch {
            facts["titleSet"] = String(describing: error)
        }
        MainActor.assumeIsolated {
            let scene = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }.first
            let window = scene?.windows.first
            let insets = window?.safeAreaInsets ?? .zero
            facts["sceneTitle"] = scene?.title ?? ""
            facts["statusBarHidden"] = scene?.statusBarManager?.isStatusBarHidden ?? false
            facts["statusBarStyle"] = scene?.statusBarManager?.statusBarStyle.rawValue ?? -1
            facts["orientation"] = scene?.interfaceOrientation.rawValue ?? 0
            facts["safeTop"] = Double(insets.top)
            facts["safeLeft"] = Double(insets.left)
            facts["safeRight"] = Double(insets.right)
            facts["scale"] = Double(window?.screen.scale ?? 0)
            if let web = window.flatMap(Self.webView) {
                facts["webOpaque"] = web.isOpaque
                facts["webBackground"] = Self.hex(web.backgroundColor)
                facts["webTop"] = Double(web.convert(CGPoint.zero, to: nil).y)
            }
        }
        let data = (try? JSONSerialization.data(withJSONObject: facts, options: [.sortedKeys])) ?? Data()
        return String(decoding: data, as: UTF8.self)
    }

    func record() -> Int {
        let directory = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
        try? Data(proof.utf8).write(to: directory.appendingPathComponent("window-probe.json"), options: .atomic)
        return 1
    }

    @MainActor
    private static func webView(_ view: UIView) -> WKWebView? {
        if let web = view as? WKWebView { return web }
        for child in view.subviews {
            if let web = webView(child) { return web }
        }
        return nil
    }

    private static func hex(_ color: UIColor?) -> String {
        var red: CGFloat = 0, green: CGFloat = 0, blue: CGFloat = 0, alpha: CGFloat = 0
        guard let color, color.getRed(&red, green: &green, blue: &blue, alpha: &alpha) else { return "none" }
        return String(format: "#%02x%02x%02x", Int((red * 255).rounded()), Int((green * 255).rounded()), Int((blue * 255).rounded()))
    }
}
`);
    return project;
  }

  static binary() {
    if (process.env.TAC_BIN) return process.env.TAC_BIN;
    const { run } = IosWindowSettingsGate;
    run('cargo', ['build', '--locked', '--bin', 'ty'], 1200000);
    return path.join(JSON.parse(run('cargo', ['metadata', '--format-version', '1', '--no-deps'])).target_directory, 'debug/ty');
  }

  static build(binary, name) {
    const project = IosWindowSettingsGate.stage(name, IosWindowSettingsGate.apps[name]);
    console.log(`Building the ${name} iOS app...`);
    IosWindowSettingsGate.run(binary, ['build', project, '--target', 'ios'], 600000);
    return path.join(project, 'dist/ios/WindowSettings.app');
  }

  static boot(deviceName, family) {
    const { simctl, run, deviceSet } = IosWindowSettingsGate;
    const runtime = JSON.parse(simctl('list', 'runtimes', '--json')).runtimes
      .filter(item => item.isAvailable && item.identifier.includes('.iOS-'))
      .sort((a, b) => b.version.localeCompare(a.version, undefined, { numeric: true }))[0];
    assert.ok(runtime, 'an installed iOS simulator runtime is required');
    const types = runtime.supportedDeviceTypes;
    const type = types.find(item => item.name === deviceName)
      ?? types.find(item => item.productFamily === family && !item.name.includes('SE'));
    assert.ok(type, `the iOS ${runtime.version} runtime needs an ${family} device type`);
    const device = simctl('create', `TachyonWindow-${randomUUID()}`, type.identifier, runtime.identifier);
    IosWindowSettingsGate.devices.push(device);
    console.log(`Booting ${type.name} with iOS ${runtime.version}...`);
    simctl('boot', device);
    run('/usr/bin/xcrun', ['simctl', '--set', deviceSet, 'bootstatus', device, '-b'], 300000);
    return device;
  }

  static async launch(device, bundle, name) {
    const { simctl, appId, pause } = IosWindowSettingsGate;
    spawnSync('/usr/bin/xcrun', ['simctl', '--set', IosWindowSettingsGate.deviceSet, 'uninstall', device, appId], { stdio: 'ignore', timeout: 60000 });
    simctl('install', device, bundle);
    simctl('launch', device, appId);
    const proof = path.join(simctl('get_app_container', device, appId, 'data'), 'Documents/window-probe.json');
    for (let attempt = 0; attempt < 120 && !existsSync(proof); attempt++) await pause(250);
    assert.ok(existsSync(proof), `the ${name} app must record its window probe`);
    await pause(500);
    const recorded = JSON.parse(readFileSync(proof, 'utf8'));
    assert.equal(recorded.error, undefined, `${name}: ${recorded.error}`);
    console.log(`${name}: ${JSON.stringify(recorded)}`);
    const screenshot = path.join(IosWindowSettingsGate.root, `${name}-${device}.png`);
    simctl('io', device, 'screenshot', screenshot);
    simctl('terminate', device, appId);
    return { ...recorded, image: IosWindowSettingsGate.readPng(screenshot) };
  }

  static readPng(file) {
    const bytes = readFileSync(file);
    let offset = 8;
    let header;
    const data = [];
    while (offset < bytes.length) {
      const length = bytes.readUInt32BE(offset);
      const type = bytes.toString('latin1', offset + 4, offset + 8);
      const body = bytes.subarray(offset + 8, offset + 8 + length);
      if (type === 'IHDR') header = { width: body.readUInt32BE(0), height: body.readUInt32BE(4), depth: body[8], color: body[9], interlace: body[12] };
      if (type === 'IDAT') data.push(body);
      offset += length + 12;
    }
    assert.ok(header && header.depth === 8 && [2, 6].includes(header.color) && header.interlace === 0, `unexpected screenshot format: ${JSON.stringify(header)}`);
    const channels = header.color === 6 ? 4 : 3;
    const stride = header.width * channels;
    const raw = inflateSync(Buffer.concat(data));
    const pixels = Buffer.alloc(stride * header.height);
    for (let row = 0; row < header.height; row++) {
      const filter = raw[row * (stride + 1)];
      const line = raw.subarray(row * (stride + 1) + 1, (row + 1) * (stride + 1));
      for (let index = 0; index < stride; index++) {
        const left = index >= channels ? pixels[row * stride + index - channels] : 0;
        const up = row > 0 ? pixels[(row - 1) * stride + index] : 0;
        const corner = row > 0 && index >= channels ? pixels[(row - 1) * stride + index - channels] : 0;
        const before = [0, left, up, (left + up) >> 1, IosWindowSettingsGate.paeth(left, up, corner)][filter];
        assert.notEqual(before, undefined, `unknown PNG filter ${filter}`);
        pixels[row * stride + index] = (line[index] + before) & 0xff;
      }
    }
    return { width: header.width, height: header.height, pixel: (x, y) => {
      const at = Math.round(y) * stride + Math.round(x) * channels;
      return [pixels[at], pixels[at + 1], pixels[at + 2]];
    } };
  }

  static paeth(left, up, corner) {
    const guess = left + up - corner;
    const [toLeft, toUp, toCorner] = [left, up, corner].map(value => Math.abs(guess - value));
    if (toLeft <= toUp && toLeft <= toCorner) return left;
    return toUp <= toCorner ? up : corner;
  }

  static rgb(hex) {
    return [1, 3, 5].map(start => Number.parseInt(hex.slice(start, start + 2), 16));
  }

  static near(pixel, hex) {
    return IosWindowSettingsGate.rgb(hex).every((channel, index) => Math.abs(channel - pixel[index]) <= 8);
  }

  static rowShare(image, y, hex) {
    let matches = 0;
    for (let x = 0; x < image.width; x++) if (IosWindowSettingsGate.near(image.pixel(x, y), hex)) matches++;
    return matches / image.width;
  }

  static checkStandard(result, bundle) {
    const { native, page } = result;
    const { styles, rowShare, near, run } = IosWindowSettingsGate;
    assert.equal(native.statusBarStyle, styles.lightContent, 'a dark theme_color gives light status bar icons');
    assert.equal(native.statusBarHidden, false);
    assert.equal(native.orientation, 1, 'a portrait app opens in portrait');
    assert.ok(native.safeTop > 0, `the iPhone has a status bar inset: ${native.safeTop}`);
    assert.equal(native.webTop, native.safeTop, 'the standard page starts below the status bar');
    assert.equal(native.webOpaque, false);
    assert.equal(native.webBackground, '#ffaa00', 'background_color colors the web view');
    assert.equal(page.titleBarHeight, '', 'a standard window sets no title bar variables');
    const statusRow = (native.safeTop - 3) * native.scale;
    assert.ok(rowShare(result.image, statusRow, '#0f6cbd') > 0.5, `theme_color fills the status bar: ${result.image.pixel(result.image.width / 2, statusRow)}`);
    const middle = result.image.pixel(result.image.width / 2, result.image.height * 0.6);
    assert.ok(near(middle, '#ffaa00'), `background_color shows through the page: ${middle}`);
    const plist = path.join(bundle, 'Info.plist');
    assert.equal(run('/usr/bin/plutil', ['-extract', 'UILaunchScreen.UIColorName', 'raw', plist]), 'TachyonLaunchBackground');
    assert.match(run('/usr/bin/xcrun', ['assetutil', '--info', path.join(bundle, 'Assets.car')]), /TachyonLaunchBackground/);
    IosWindowSettingsGate.checkTitle(native, 'iPhone');
  }

  static checkTitle(native, device) {
    assert.equal(native.grantTitle, true, `${device}: linked Swift sees the title grant`);
    assert.equal(native.grantMinimize, false, `${device}: linked Swift sees no minimize grant`);
    assert.equal(native.titleSet, 'yes', `${device}: linked Swift sets the title`);
    assert.equal(native.sceneTitle, 'Quarterly report', `${device}: the scene title`);
  }

  static checkIntegrated(result) {
    const { native, page } = result;
    const { styles, rowShare } = IosWindowSettingsGate;
    assert.equal(native.statusBarStyle, styles.darkContent, 'a light theme_color gives dark status bar icons');
    assert.ok(native.safeTop > 0);
    assert.equal(native.webTop, 0, 'the integrated page starts at the top edge');
    assert.equal(page.titleBarHeight, `${Math.round(native.safeTop)}px`, 'the title bar height is the safe area top');
    assert.equal(page.leadingInset, `${Math.round(native.safeLeft)}px`);
    assert.equal(page.trailingInset, `${Math.round(native.safeRight)}px`);
    assert.deepEqual(JSON.parse(page.insetsAnswer), { height: Math.round(native.safeTop), leading: Math.round(native.safeLeft), trailing: Math.round(native.safeRight) });
    assert.equal(page.barHeight, Math.round(native.safeTop), 'the page sizes its bar from --tac-titlebar-height');
    const statusRow = (native.safeTop - 3) * native.scale;
    assert.ok(rowShare(result.image, statusRow, '#ff00ff') > 0.5, `the page draws under the status bar: ${result.image.pixel(result.image.width / 2, statusRow)}`);
  }

  static checkFullscreen(result) {
    const { native, page } = result;
    assert.equal(native.statusBarHidden, true, 'display fullscreen hides the status bar');
    assert.ok([3, 4].includes(native.orientation), `a landscape app opens in landscape: ${native.orientation}`);
    assert.ok(page.width > page.height, `the page is wider than tall: ${page.width}x${page.height}`);
    assert.match(page.insetsAnswer, /refused|not implement/, 'a standard window answers no title bar insets');
  }

  static keep(file, name) {
    mkdirSync(IosWindowSettingsGate.evidence, { recursive: true });
    copyFileSync(file, path.join(IosWindowSettingsGate.evidence, name));
  }

  static async checkLaunchScreen(device, bundle, native) {
    const { simctl, appId, pause, rowShare, readPng, root, keep } = IosWindowSettingsGate;
    spawnSync('/usr/bin/xcrun', ['simctl', '--set', IosWindowSettingsGate.deviceSet, 'uninstall', device, appId], { stdio: 'ignore', timeout: 60000 });
    simctl('install', device, bundle);
    const launched = simctl('launch', '--wait-for-debugger', device, appId);
    const pid = Number(launched.match(/:\s*(\d+)\s*$/)?.[1]);
    const proof = path.join(simctl('get_app_container', device, appId, 'data'), 'Documents/window-probe.json');
    const statusRow = (native.safeTop - 3) * native.scale;
    let shares = [];
    try {
      for (let attempt = 0; attempt < 20; attempt++) {
        await pause(500);
        const file = path.join(root, `launch-screen-${attempt}.png`);
        simctl('io', device, 'screenshot', file);
        const image = readPng(file);
        shares = [statusRow, image.height * 0.3, image.height * 0.6, image.height * 0.9].map(y => rowShare(image, y, '#ffaa00'));
        if (shares[0] > 0.5 && shares.slice(1).every(share => share > 0.9)) {
          keep(file, 'launch-screen.png');
          break;
        }
      }
      assert.ok(shares[0] > 0.5 && shares.slice(1).every(share => share > 0.9), `the launch screen shows background_color behind the status bar and across the screen before the app runs: ${shares}`);
      assert.ok(!existsSync(proof), 'the app had not run its page while the launch screen showed');
    } finally {
      try { process.kill(pid, 'SIGKILL'); } catch { /* already ended */ }
      spawnSync('/usr/bin/xcrun', ['simctl', '--set', IosWindowSettingsGate.deviceSet, 'terminate', device, appId], { stdio: 'ignore', timeout: 30000 });
    }
    console.log(`launch screen: background_color #ffaa00 filled the screen (row shares ${shares.map(share => share.toFixed(2)).join(', ')}) while the process ${pid} waited before main`);
  }

  static buildRefusal(binary) {
    const project = path.join(IosWindowSettingsGate.root, 'refusal');
    const application = PageServicesFixture.stageWindowRefusal(project, IosWindowSettingsGate.refusalId);
    console.log('Building the window refusal iOS app...');
    IosWindowSettingsGate.run(binary, ['build', project, '--target', 'ios'], 600000);
    IosWindowSettingsGate.run('/usr/bin/xcrun', ['swiftc', '-parse-as-library', '-O', path.join(IosWindowSettingsGate.repo, 'scripts/ios/screenshot-text.swift'), '-o', IosWindowSettingsGate.reader], 600000);
    return path.join(project, 'dist/ios', `${application}.app`);
  }

  static squeeze(text) {
    return text.toLowerCase().replace(/\s+/g, '');
  }

  static async checkRefusal(device, bundle) {
    const { simctl, refusalId, pause, root, run, reader, squeeze, keep } = IosWindowSettingsGate;
    const { invalid, oversized } = PageServicesFixture.refusedTitles;
    simctl('install', device, bundle);
    simctl('launch', device, refusalId);
    let shown = '';
    try {
      for (let attempt = 0; attempt < 90 && !squeeze(shown).includes('finished'); attempt++) {
        await pause(1000);
        const file = path.join(root, 'refusal.png');
        simctl('io', device, 'screenshot', file);
        shown = JSON.parse(run(reader, [file])).sort((a, b) => a.y - b.y).map(({ text }) => text).join(' ');
        keep(file, 'window-refusal.png');
      }
    } finally {
      spawnSync('/usr/bin/xcrun', ['simctl', '--set', IosWindowSettingsGate.deviceSet, 'terminate', device, refusalId], { stdio: 'ignore', timeout: 30000 });
    }
    assert.ok(squeeze(shown).includes(squeeze(`Heard 2 ${invalid} ${oversized} finished`)), `a JavaScript caller hears the iOS host refuse a 2000-byte title and an oversized title, and nothing for a valid title: ${shown}`);
    console.log(`window refusal: ${shown}`);
  }

  static cleanup() {
    for (const device of IosWindowSettingsGate.devices) {
      for (const args of [['shutdown', device], ['delete', device]]) {
        spawnSync('/usr/bin/xcrun', ['simctl', '--set', IosWindowSettingsGate.deviceSet, ...args], { timeout: 60000, stdio: 'ignore' });
      }
    }
    rmSync(IosWindowSettingsGate.root, { recursive: true, force: true });
  }

  static async main() {
    try {
      mkdirSync(IosWindowSettingsGate.deviceSet);
      const binary = IosWindowSettingsGate.binary();
      const bundles = Object.fromEntries(Object.keys(IosWindowSettingsGate.apps).map(name => [name, IosWindowSettingsGate.build(binary, name)]));
      const refusal = IosWindowSettingsGate.buildRefusal(binary);
      const phone = IosWindowSettingsGate.boot('iPhone 17 Pro', 'iPhone');
      const standard = await IosWindowSettingsGate.launch(phone, bundles.standard, 'standard');
      IosWindowSettingsGate.checkStandard(standard, bundles.standard);
      await IosWindowSettingsGate.checkLaunchScreen(phone, bundles.standard, standard.native);
      await IosWindowSettingsGate.checkRefusal(phone, refusal);
      IosWindowSettingsGate.checkIntegrated(await IosWindowSettingsGate.launch(phone, bundles.integrated, 'integrated'));
      IosWindowSettingsGate.checkFullscreen(await IosWindowSettingsGate.launch(phone, bundles.fullscreen, 'fullscreen'));
      const tablet = IosWindowSettingsGate.boot('iPad Pro 13-inch (M5) (16GB)', 'iPad');
      IosWindowSettingsGate.checkTitle((await IosWindowSettingsGate.launch(tablet, bundles.standard, 'standard')).native, 'iPad');
      console.log('PASS: iOS window settings: theme_color status bar and icons, background_color web view and launch screen (asset catalog and on screen before main), integrated title bar insets, fullscreen, orientation, the linked Swift scene title on iPhone and iPad, and host title refusals heard as BrowserCallFailed');
    } finally {
      IosWindowSettingsGate.cleanup();
    }
  }
}

process.once('SIGINT', () => { IosWindowSettingsGate.cleanup(); process.exit(130); });
await IosWindowSettingsGate.main();
