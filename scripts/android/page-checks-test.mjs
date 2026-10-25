#!/usr/bin/env node
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { PageChecksFixture } from '../native/page-checks-fixture.mjs';

class AndroidPageChecksGate {
  static repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
  static sdk = process.env.ANDROID_HOME ?? process.env.ANDROID_SDK_ROOT;
  static adb = path.join(AndroidPageChecksGate.sdk ?? '', 'platform-tools', process.platform === 'win32' ? 'adb.exe' : 'adb');
  static packageName = 'dev.tachyon.page_checks';
  static apk = 'dist/android/PageChecks/PageChecks.apk';
  static appOrigin = 'https://appassets.tachyon.local';
  static storageCommitDelayMs = 6000;
  static scopedColor = 'rgb(10, 20, 30)';
  static styleFloor = 118;
  static links = [
    { name: 'Open the https link', url: 'https://example.com/tachyon-https-link', opens: 'com.android.chrome', logged: 'dat=https://example.com/' },
    { name: 'Open the http link', url: 'http://example.org/tachyon-http-link', opens: 'com.android.chrome', logged: 'dat=http://example.org/' },
    { name: 'Write a mail', url: 'mailto:ada@example.com?subject=Tachyon', opens: 'com.google.android.gm', logged: 'dat=mailto:' },
  ];
  static userNameLink = { name: 'Open the link with a user name', url: 'https://ada@example.net/tachyon-user-name', logged: 'example.net' };
  static files = {
    'client/pages/tac.html': '<state-panel hydrate="load" />\n<note-panel hydrate="load" />\n<theme-panel hydrate="load" />\n<reach-panel hydrate="load" />\n<visit-panel hydrate="load" />\n<link-panel />\n',
    ...PageChecksFixture.visitFiles,
    'client/components/state/panel/tac.html': '<section aria-label="Application state"><output aria-label="Page heard">{pageState.heard.join(",")}</output><output aria-label="Native heard">{nativeState.heard}</output></section>\n',
    'client/components/note/panel/tac.html': '<section aria-label="Notes"><button on:click="note.read()">Read the note</button><button on:click="note.readFresh()">Read the fresh note</button><output aria-label="Note">{note.text}</output><output aria-label="Fresh note">{note.fresh}</output></section>\n',
    'client/components/theme/panel/tac.html': '<section aria-label="Theme"><button on:click="theme.save()">Save the dark theme</button><output aria-label="Saved theme">{theme.saved}</output></section>\n',
    'client/components/theme/panel/tac.css': `output { color: ${AndroidPageChecksGate.scopedColor}; }\n`,
    'client/services/PageStateService.js': `@Service
export default class PageStateService {
  heard = [];

  /** @param {ApplicationStateChanged} changed */
  @OnEvent({"replay":true})
  applicationStateChanged(changed) {
    this.heard = [...this.heard, changed.state];
  }
}
`,
    'client/services/NativeStateService.kt': `@Service
class NativeStateService {
    var heard: String = ""

    @OnEvent({"replay":true})
    fun applicationStateChanged(changed: ApplicationStateChanged) {
        heard = joined(changed.state)
    }

    private fun joined(state: String): String = if (heard.isEmpty()) state else heard + "," + state
}
`,
    'client/remotes/NoteClient.js': `@HTTPClient
@CacheSettings({"name":"notes","ttlMs":1,"maxBytes":4096,"maxEntries":4})
export default class NoteClient {
  /** @returns {Promise<{ text: string }>} */
  @Cached
  @GetRequest("/api/notes")
  async read() {}

  /** @returns {Promise<{ text: string }>} */
  @GetRequest("/api/fresh")
  async fresh() {}
}
`,
    'client/services/NoteService.js': `@Service
export default class NoteService {
  /** @type {NoteClient} */
  @Needs
  noteClient;
  reads = 0;
  freshReads = 0;
  text = 'No read yet';
  fresh = 'No fresh read yet';

  async read() {
    this.reads += 1;
    const label = \`Read \${this.reads}\`;
    this.text = await this.noteClient.read().then((note) => \`\${label}: \${note.text}\`, () => \`\${label} failed\`);
  }

  async readFresh() {
    this.freshReads += 1;
    const label = \`Fresh read \${this.freshReads}\`;
    this.fresh = await this.noteClient.fresh().then((note) => \`\${label}: \${note.text}\`, (problem) => \`\${label} failed: \${problem.message}\`);
  }
}
`,
    'client/services/ThemeService.js': `@Service
export default class ThemeService {
  static KEY = 'theme';
  static DARK = 'dark';
  /** @type {PreferenceStrings} */
  @Needs
  preferenceStrings;
  saved = 'none';

  @OnStart
  load() {
    this.saved = this.#shown(this.preferenceStrings.read(ThemeService.KEY));
  }

  save() {
    this.preferenceStrings.write(ThemeService.KEY, ThemeService.DARK);
    this.saved = this.#shown(this.preferenceStrings.read(ThemeService.KEY));
  }

  /**
   * @param {string | null} value
   * @returns {string}
   */
  #shown(value) {
    return value ?? 'none';
  }
}
`,
  };

  static run(command, args, options = {}) {
    const result = spawnSync(command, args, { cwd: AndroidPageChecksGate.repo, encoding: 'utf8', timeout: 240000, maxBuffer: 8 * 1024 * 1024, ...options });
    assert.equal(result.status, 0, `${command} ${args.join(' ')}: ${result.error || result.stderr || result.stdout}`);
    return result.stdout.trim();
  }

  static adbShell(...args) {
    return AndroidPageChecksGate.run(AndroidPageChecksGate.adb, ['shell', ...args]);
  }

  static pause(milliseconds) {
    return new Promise(resolve => setTimeout(resolve, milliseconds));
  }

  static write(project, name, contents) {
    const file = path.join(project, name);
    mkdirSync(path.dirname(file), { recursive: true });
    writeFileSync(file, contents);
  }

  static linkView() {
    const { links, userNameLink } = AndroidPageChecksGate;
    return `<nav aria-label="Links">${[...links, userNameLink].map(link => `<p><a href="${link.url}">${link.name}</a></p>`).join('')}</nav>\n`;
  }

  static stage(project, yonOrigin, undeclaredOrigin) {
    const { write, files, linkView } = AndroidPageChecksGate;
    const addresses = { '/undeclared': `${undeclaredOrigin}/probe`, '/yon': `${yonOrigin}/api/fresh` };
    for (const [name, contents] of Object.entries({ ...files, ...PageChecksFixture.reachFiles(addresses) })) write(project, name, contents);
    write(project, 'client/components/link/panel/tac.html', linkView());
    write(project, 'client/manifest.json', JSON.stringify({
      name: 'Page Checks',
      start_url: '/',
      tac: {
        application: { id: 'dev.tachyon.page-checks', version: '0.1.0', build: 1 },
        visibility: { check: false },
        yon: { origin: yonOrigin },
        cache: [{ path: '/api/notes', policy: 'network-first' }, { path: '/api/kept', policy: 'cache-first' }],
        browserApis: ['preferenceStrings'],
      },
    }));
  }

  static binary() {
    if (process.env.TAC_BIN) return process.env.TAC_BIN;
    const run = AndroidPageChecksGate.run;
    run('cargo', ['build', '--locked', '--bin', 'ty']);
    return path.join(JSON.parse(run('cargo', ['metadata', '--format-version', '1', '--no-deps'])).target_directory, 'debug/ty');
  }

  static hostSources(folder) {
    return readdirSync(folder, { recursive: true }).filter(name => name.endsWith('.java')).map(name => readFileSync(path.join(folder, name), 'utf8'));
  }

  static checkPolicy(project, yonOrigin, undeclaredOrigin) {
    const source = AndroidPageChecksGate.hostSources(path.join(project, 'dist/android')).find(text => text.includes('String CONTENT_SECURITY_POLICY ='));
    assert.ok(source, 'The Android host source has a CONTENT_SECURITY_POLICY');
    const policy = source.match(/String CONTENT_SECURITY_POLICY = "[^"]*connect-src [^;"]*/)?.[0] ?? '';
    assert.ok(policy.includes(yonOrigin), `connect-src lacks the Yon origin ${yonOrigin}: ${policy}`);
    assert.ok(!policy.includes(undeclaredOrigin), `connect-src lists the undeclared ${undeclaredOrigin}: ${policy}`);
  }

  static checkBundle(project) {
    const host = JSON.parse(readFileSync(path.join(project, 'dist/android/tachyon.host.json'), 'utf8'));
    assert.deepEqual(host.companions, [{ service: 'nativeState', language: 'Kotlin' }]);
    const bundle = path.join(project, 'dist/android/web');
    assert.ok(existsSync(path.join(bundle, '.tachyon/tac-yon-reads.js')), 'The bundle carries the tac.cache rules for Yon reads');
    assert.match(readFileSync(path.join(bundle, 'index.html'), 'utf8'), /tachyon-browser-apis" content="preferenceStrings"/);
  }

  static yon() {
    const { appOrigin } = AndroidPageChecksGate;
    const yon = { requests: [], answered: { '/api/notes': 0, '/api/kept': 0, '/api/fresh': 0 } };
    yon.server = createServer((request, response) => {
      const pathname = new URL(request.url, 'http://yon').pathname;
      yon.requests.push(`${request.method} ${pathname}`);
      const known = pathname in yon.answered;
      if (known && request.method === 'GET') yon.answered[pathname] += 1;
      const text = { '/api/notes': `Note ${yon.answered['/api/notes']} from Yon`, '/api/kept': `Kept ${yon.answered['/api/kept']} from Yon`, '/api/fresh': 'Fresh from Yon' }[pathname];
      response.writeHead(known ? 200 : 404, {
        'content-type': 'application/json',
        'cache-control': pathname === '/api/fresh' ? 'no-store' : 'public, no-cache',
        'access-control-allow-origin': appOrigin,
        vary: 'origin',
        connection: 'close',
      });
      response.end(JSON.stringify({ text }));
    });
    return new Promise(resolve => yon.server.listen(0, '127.0.0.1', () => resolve({ ...yon, port: yon.server.address().port })));
  }

  static async launch() {
    const { adbShell, packageName, pause } = AndroidPageChecksGate;
    adbShell('am', 'start', '-W', '-n', `${packageName}/.MainActivity`);
    let pid = '';
    for (let attempt = 0; attempt < 60; attempt++) {
      pid = spawnSync(AndroidPageChecksGate.adb, ['shell', 'pidof', packageName], { encoding: 'utf8', timeout: 60000 }).stdout.trim();
      if (pid && adbShell('cat', '/proc/net/unix').includes(`webview_devtools_remote_${pid}`)) break;
      await pause(500);
    }
    assert.match(pid, /^\d+$/, 'The page checks app must start');
    return pid;
  }

  static async connect(pid) {
    const port = AndroidPageChecksGate.run(AndroidPageChecksGate.adb, ['forward', 'tcp:0', `localabstract:webview_devtools_remote_${pid}`]);
    const browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`, { noDefaults: true });
    const page = browser.contexts()[0].pages()[0];
    assert.ok(page, 'Android WebView exposes a live document');
    page.on('pageerror', error => console.error('Android page error:', error.message));
    await page.getByRole('button', { name: 'Read the note' }).waitFor();
    return { port, browser, page };
  }

  static async disconnect(connection) {
    if (!connection) return;
    await connection.browser.close();
    AndroidPageChecksGate.run(AndroidPageChecksGate.adb, ['forward', '--remove', `tcp:${connection.port}`]);
  }

  static resumedActivity() {
    return AndroidPageChecksGate.adbShell('dumpsys', 'activity', 'activities').split('\n').find(line => line.includes('topResumedActivity=')) ?? '';
  }

  static async waitFor(condition, message) {
    for (let attempt = 0; attempt < 80; attempt++) {
      if (await condition()) return;
      await AndroidPageChecksGate.pause(250);
    }
    assert.fail(typeof message === 'function' ? await message() : message);
  }

  static output(page, name) {
    return page.getByRole('status', { name, exact: true }).textContent();
  }

  static nativeHeard(page) {
    return page.evaluate(async () => await __tachyonTac.service('nativeState').heard);
  }

  static async bringBack() {
    const { adbShell, packageName, resumedActivity, waitFor } = AndroidPageChecksGate;
    await waitFor(() => {
      if (resumedActivity().includes(packageName)) return true;
      adbShell('am', 'start', '-n', `${packageName}/.MainActivity`);
      return false;
    }, () => `The app comes back to the front: ${resumedActivity()}`);
  }

  static async tapLink(name) {
    const { adb, adbShell, repo, waitFor } = AndroidPageChecksGate;
    let bounds;
    await waitFor(() => {
      const dump = spawnSync(adb, ['shell', 'uiautomator', 'dump', '/sdcard/tachyon-page-checks.xml'], { cwd: repo, encoding: 'utf8', timeout: 60000 });
      if (!`${dump.stdout}${dump.stderr}`.includes('dumped to')) return false;
      const screen = adbShell('cat', '/sdcard/tachyon-page-checks.xml');
      bounds = screen.match(new RegExp(`<node [^>]*(?:text|content-desc)="${name}"[^>]*bounds="\\[(\\d+),(\\d+)\\]\\[(\\d+),(\\d+)\\]"`));
      return Boolean(bounds);
    }, `The page shows the link ${name}`);
    const [, left, top, right, bottom] = bounds.map(Number);
    adbShell('input', 'tap', String(Math.round((left + right) / 2)), String(Math.round((top + bottom) / 2)));
  }

  static applicationUid() {
    const { adbShell, packageName } = AndroidPageChecksGate;
    return adbShell('pm', 'list', 'packages', '-U', packageName).match(/uid:(\d+)/)?.[1];
  }

  static async checkApplicationState(page) {
    const { adbShell, output, nativeHeard, waitFor, resumedActivity, packageName, bringBack } = AndroidPageChecksGate;
    await waitFor(async () => await output(page, 'Page heard') === 'foreground' && await nativeHeard(page) === 'foreground',
      async () => `The page and the Kotlin service hear the replayed foreground state: ${await output(page, 'Page heard')} / ${await nativeHeard(page)}`);
    adbShell('input', 'keyevent', 'KEYCODE_HOME');
    await waitFor(() => !resumedActivity().includes(packageName), () => `Home moves the app behind the launcher: ${resumedActivity()}`);
    await waitFor(async () => await output(page, 'Page heard') === 'foreground,background' && await nativeHeard(page) === 'foreground,background',
      async () => `onStop reaches the page and the Kotlin service as background: ${await output(page, 'Page heard')} / ${await nativeHeard(page)}`);
    await bringBack();
    await waitFor(async () => await output(page, 'Page heard') === 'foreground,background,foreground' && await nativeHeard(page) === 'foreground,background,foreground',
      async () => `onStart reaches the page and the Kotlin service as foreground: ${await output(page, 'Page heard')} / ${await nativeHeard(page)}`);
    console.log('PASS: Android ApplicationStateChanged: onStop and onStart reached a page service and a linked Kotlin service as background and foreground');
  }

  static readKept(page, yonOrigin) {
    return page.evaluate(async url => {
      try {
        const response = await globalThis[Symbol.for('tachyon.tac.yon-reads')].fetch(url, { credentials: 'omit' });
        return (await response.json()).text;
      } catch (error) { return `refused: ${error.name}`; }
    }, `${yonOrigin}/api/kept`);
  }

  static airplaneMode(state) {
    AndroidPageChecksGate.adbShell('cmd', 'connectivity', 'airplane-mode', state);
  }

  static reverse(port, state) {
    const { run, adb } = AndroidPageChecksGate;
    run(adb, state === 'enable' ? ['reverse', `tcp:${port}`, `tcp:${port}`] : ['reverse', '--remove', `tcp:${port}`]);
  }

  static async checkOfflineReads(page, yon, yonOrigin) {
    const { output, waitFor, readKept, airplaneMode, reverse, adbShell } = AndroidPageChecksGate;
    const press = name => page.getByRole('button', { name }).click();
    await press('Read the note');
    await waitFor(async () => await output(page, 'Note') === 'Read 1: Note 1 from Yon', async () => `The network-first read reaches Yon online: ${await output(page, 'Note')}`);
    await press('Read the fresh note');
    await waitFor(async () => await output(page, 'Fresh note') === 'Fresh read 1: Fresh from Yon', async () => `A read without a rule reaches Yon online: ${await output(page, 'Fresh note')}`);
    assert.equal(await readKept(page, yonOrigin), 'Kept 1 from Yon', 'The cache-first read reaches Yon the first time');
    assert.equal(await readKept(page, yonOrigin), 'Kept 1 from Yon', 'The second cache-first read answers from the cache');
    assert.equal(yon.answered['/api/kept'], 1, 'The second cache-first read does not reach Yon');
    const online = yon.requests.length;
    let refused = '';
    airplaneMode('enable');
    reverse(yon.port, 'disable');
    yon.server.closeAllConnections();
    try {
      await waitFor(() => adbShell('settings', 'get', 'global', 'airplane_mode_on') === '1', 'Airplane mode turns on');
      await press('Read the fresh note');
      await waitFor(async () => (await output(page, 'Fresh note')).startsWith('Fresh read 2 failed'), async () => `A read without a rule fails in airplane mode: ${await output(page, 'Fresh note')}; Yon saw ${yon.requests.slice(online).join(', ') || 'nothing'}`);
      refused = await output(page, 'Fresh note');
      await press('Read the note');
      await waitFor(async () => await output(page, 'Note') === 'Read 2: Note 1 from Yon', async () => `The network-first read answers its kept copy in airplane mode: ${await output(page, 'Note')}`);
      assert.equal(await readKept(page, yonOrigin), 'Kept 1 from Yon', 'The cache-first read answers its kept copy in airplane mode');
      assert.deepEqual(yon.requests.slice(online), [], 'No read reached Yon in airplane mode');
    } finally {
      airplaneMode('disable');
      reverse(yon.port, 'enable');
    }
    await waitFor(() => adbShell('settings', 'get', 'global', 'airplane_mode_on') === '0', 'Airplane mode turns off');
    console.log(`PASS: Android tac.cache: in airplane mode, with the adb reverse route to the local Yon removed, a network-first and a cache-first Yon read answered from Cache Storage, a read without a rule failed (${refused}), and Yon saw no request`);
  }

  static startedViews() {
    return AndroidPageChecksGate.run(AndroidPageChecksGate.adb, ['logcat', '-d', '-s', 'ActivityTaskManager:I']).split('\n').filter(line => line.includes('START u0') && line.includes('act=android.intent.action.VIEW'));
  }

  static async checkExternalLinks(page) {
    const { run, adb, links, userNameLink, startedViews, waitFor, resumedActivity, packageName, bringBack, pause, applicationUid, tapLink, adbShell } = AndroidPageChecksGate;
    const uid = applicationUid();
    assert.match(uid, /^\d+$/, 'The app has a uid');
    for (const link of links) {
      run(adb, ['logcat', '-c']);
      await tapLink(link.name);
      await waitFor(() => resumedActivity().includes(link.opens), () => `${link.url} opens ${link.opens}: ${resumedActivity()}`);
      const started = startedViews();
      assert.ok(started.some(line => line.includes(link.logged) && line.includes(`from uid ${uid} `)), `${link.url} starts ACTION_VIEW from the app (uid ${uid}): ${started.join(' | ')}`);
      await pause(2000);
      adbShell('am', 'force-stop', link.opens);
      await bringBack();
      assert.equal(new URL(page.url()).origin, AndroidPageChecksGate.appOrigin, `${link.url} leaves the page where it was`);
    }
    run(adb, ['logcat', '-c']);
    await tapLink(userNameLink.name);
    await pause(3000);
    assert.ok(resumedActivity().includes(packageName), `A link with a user name opens nothing: ${resumedActivity()}`);
    assert.deepEqual(startedViews().filter(line => line.includes(`from uid ${uid} `)), [], 'A link with a user name starts no ACTION_VIEW');
    console.log(`PASS: Android external links: ${links.map(link => link.url.split(':')[0]).join(', ')} links opened through ACTION_VIEW in ${[...new Set(links.map(link => link.opens))].join(' and ')}, and a link with a user name opened nothing`);
  }

  static async undeclaredHost() {
    const host = { requests: [] };
    host.server = createServer((request, response) => {
      host.requests.push(`${request.method} ${request.url} from ${request.headers.origin ?? 'no origin'}`);
      response.writeHead(200, { 'content-type': 'application/json', 'access-control-allow-origin': '*', connection: 'close' });
      response.end('{"reached":true}');
    });
    return new Promise(resolve => host.server.listen(0, '127.0.0.1', () => resolve({ ...host, port: host.server.address().port })));
  }

  static async checkReach(page, undeclared) {
    const { waitFor } = AndroidPageChecksGate;
    const shown = text => page.getByText(text, { exact: true }).count().then(count => count > 0);
    await page.getByRole('button', { name: 'Reach the undeclared host' }).click();
    await waitFor(() => shown('Undeclared host refused by connect-src'), async () => `connect-src refuses the undeclared host: ${await page.locator('body').innerText().catch(() => 'no page text')}`);
    await page.getByRole('button', { name: 'Reach the Yon origin' }).click();
    await waitFor(() => shown('Yon origin reached 200'), async () => `the Yon origin answers: ${await page.locator('body').innerText().catch(() => 'no page text')}`);
    assert.deepEqual(undeclared.requests, [], `The undeclared host saw requests: ${undeclared.requests.join(', ')}`);
    console.log(`PASS: Android tac.outgoing: connect-src refused a page fetch to http://localhost:${undeclared.port} (outside tac.outgoing.hosts, reachable through adb reverse) with a securitypolicyviolation, the host saw no request, and the Yon origin answered`);
  }

  static async checkScopedStyle(page) {
    const { scopedColor, styleFloor } = AndroidPageChecksGate;
    const seen = await page.evaluate(() => ({
      inside: getComputedStyle(document.querySelector('output[aria-label="Saved theme"]')).color,
      outside: getComputedStyle(document.querySelector('output[aria-label="Note"]')).color,
      agent: navigator.userAgent,
    }));
    const engine = Number(seen.agent.match(/Chrome\/(\d+)/)?.[1]);
    assert.ok(engine >= styleFloor, `The emulator runs Android System WebView ${styleFloor} or newer: ${seen.agent}`);
    assert.equal(seen.inside, scopedColor, 'The component tac.css rule applies inside its component');
    assert.notEqual(seen.outside, scopedColor, 'The component tac.css rule stays inside its component');
    console.log(`PASS: Android scoped component styles: on Android System WebView ${engine}, a component tac.css rule coloured the output inside its component and not an output in another component`);
    return engine;
  }

  static async checkSessionState(page) {
    const { waitFor } = AndroidPageChecksGate;
    const shown = text => page.getByText(text, { exact: true }).count().then(count => count > 0);
    await waitFor(() => shown('Visit note none'), 'A fresh page shows no visit note');
    await page.getByRole('button', { name: 'Remember this visit' }).click();
    await waitFor(() => shown('Visit note remembered'), 'The @SessionState field takes the new value');
    await page.getByRole('link', { name: 'Open the other page' }).click();
    await waitFor(() => shown('The other page'), 'The link opens the other page');
    await page.getByRole('link', { name: 'Back to the first page' }).click();
    await waitFor(() => shown('Visit note remembered'), async () => `The @SessionState field is kept across the navigation: ${await page.locator('body').innerText().catch(() => 'no page text')}`);
    assert.equal(new URL(page.url()).pathname, '/', 'The page is back on the first page');
    console.log('PASS: Android @SessionState: a field set on the first page was kept across a navigation to another page and back');
  }

  static async quit() {
    const { adbShell, packageName, pause } = AndroidPageChecksGate;
    adbShell('input', 'keyevent', 'KEYCODE_HOME');
    await pause(AndroidPageChecksGate.storageCommitDelayMs);
    adbShell('am', 'force-stop', packageName);
    assert.equal(spawnSync(AndroidPageChecksGate.adb, ['shell', 'pidof', packageName], { encoding: 'utf8' }).stdout.trim(), '', 'The app process ends before the relaunch');
  }

  static async main() {
    const gate = AndroidPageChecksGate;
    assert.ok(gate.sdk, 'ANDROID_HOME or ANDROID_SDK_ROOT is required');
    const project = mkdtempSync(path.join(tmpdir(), 'tachyon-android-page-checks-'));
    const yon = await gate.yon();
    const yonOrigin = `http://localhost:${yon.port}`;
    const undeclared = await gate.undeclaredHost();
    const undeclaredOrigin = `http://localhost:${undeclared.port}`;
    let connection;
    let reversed = false;
    try {
      gate.stage(project, yonOrigin, undeclaredOrigin);
      const built = spawnSync(gate.binary(), ['build', project, '--target', 'android'], { cwd: gate.repo, encoding: 'utf8', timeout: 900000, maxBuffer: 8 * 1024 * 1024 });
      assert.equal(built.status, 0, `ty build --target android: ${built.error || built.stderr || built.stdout}`);
      assert.match(`${built.stdout}${built.stderr}`, /Android System WebView 118/, 'The build names the web view release that runs component styles (ADR 0108)');
      gate.checkBundle(project);
      gate.checkPolicy(project, yonOrigin, undeclaredOrigin);
      gate.run(gate.adb, ['wait-for-device']);
      gate.reverse(yon.port, 'enable');
      gate.reverse(undeclared.port, 'enable');
      reversed = true;
      gate.run(gate.adb, ['install', '-r', path.join(project, gate.apk)]);
      gate.adbShell('pm', 'clear', gate.packageName);
      const pid = await gate.launch();
      connection = await gate.connect(pid);
      await gate.checkApplicationState(connection.page);
      await connection.page.getByRole('button', { name: 'Save the dark theme' }).click();
      await gate.waitFor(async () => await gate.output(connection.page, 'Saved theme') === 'dark', 'preferenceStrings.write keeps the theme');
      await gate.checkOfflineReads(connection.page, yon, yonOrigin);
      await gate.checkExternalLinks(connection.page);
      await gate.checkReach(connection.page, undeclared);
      await gate.checkScopedStyle(connection.page);
      await gate.checkSessionState(connection.page);
      await gate.disconnect(connection);
      connection = undefined;
      await gate.quit();
      const relaunched = await gate.launch();
      assert.notEqual(relaunched, pid, 'The relaunch starts a new app process');
      connection = await gate.connect(relaunched);
      await gate.waitFor(async () => await gate.output(connection.page, 'Saved theme') === 'dark', async () => `preferenceStrings reads the theme back after a relaunch: ${await gate.output(connection.page, 'Saved theme')}`);
      assert.equal(await connection.page.getByText('Visit note none', { exact: true }).count(), 1, 'The @SessionState field is gone after the app was stopped and relaunched');
      console.log(`PASS: Android preferenceStrings: a write was read back after the app was stopped and relaunched (process ${pid} -> ${relaunched})`);
    } finally {
      await gate.disconnect(connection).catch(() => {});
      spawnSync(gate.adb, ['shell', 'cmd', 'connectivity', 'airplane-mode', 'disable'], { encoding: 'utf8', timeout: 60000 });
      spawnSync(gate.adb, ['shell', 'am', 'force-stop', gate.packageName], { encoding: 'utf8', timeout: 60000 });
      if (reversed) for (const port of [yon.port, undeclared.port]) spawnSync(gate.adb, ['reverse', '--remove', `tcp:${port}`], { encoding: 'utf8', timeout: 60000 });
      for (const opened of new Set(gate.links.map(link => link.opens))) spawnSync(gate.adb, ['shell', 'am', 'force-stop', opened], { encoding: 'utf8', timeout: 60000 });
      yon.server.close();
      undeclared.server.close();
      rmSync(project, { recursive: true, force: true });
    }
  }
}

await AndroidPageChecksGate.main();
