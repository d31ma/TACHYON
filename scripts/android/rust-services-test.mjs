#!/usr/bin/env node
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

class AndroidRustGate {
  static repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
  static sdk = process.env.ANDROID_HOME ?? process.env.ANDROID_SDK_ROOT;
  static adb = path.join(AndroidRustGate.sdk ?? '', 'platform-tools', process.platform === 'win32' ? 'adb.exe' : 'adb');
  static packageName = 'dev.tachyon.rust_probe';
  static apk = 'dist/android/RustProbe/RustProbe.apk';
  static abis = ['arm64-v8a', 'x86_64'];
  static release = { password: 'tachyon-gate-only', alias: 'upload', signer: 'CN=Tachyon Release Gate', bundle: 'dist-release/android/RustProbe' };
  static rustService = `use crate::models::RustNote;

#[Service]
#[derive(Default)]
struct RustCounterService {
    count: i64,
    received: String,
    #[LocalState]
    visits: i64,
    window_controls: WindowControls,
    application_event_publisher: ApplicationEventPublisher,
}

impl RustCounterService {
    pub fn increase(&mut self) -> i64 {
        self.count += 21;
        self.count
    }

    pub fn process_id(&self) -> i64 {
        i64::from(std::process::id())
    }

    pub fn visit(&mut self) -> i64 {
        self.visits += 1;
        self.visits
    }

    pub fn remember(&self) {
        tac_set_local("probe", "kept");
    }

    pub fn recall(&self) -> String {
        tac_local("probe", "missing")
    }

    pub fn grants(&self) -> String {
        let granted = |control: &str| self.window_controls.granted(control).unwrap_or(false);
        format!("title={} close={}", granted("title"), granted("close"))
    }

    pub fn rename(&self) -> String {
        self.window_controls.set_title("Rust titled").err().unwrap_or_default()
    }

    pub fn announce(&self) -> String {
        self.application_event_publisher.publish_event(RustNote { text: String::from("Rust") }, false);
        String::from("Rust")
    }

    #[OnEvent]
    fn receive(&mut self, value: RustNote) {
        self.received = value.text;
    }

    pub async fn later(&mut self) -> String {
        let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let mut started = false;
        std::future::poll_fn(|context| {
            if done.load(std::sync::atomic::Ordering::SeqCst) {
                return std::task::Poll::Ready(());
            }
            if !started {
                started = true;
                let waker = context.waker().clone();
                let flag = done.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(300));
                    flag.store(true, std::sync::atomic::Ordering::SeqCst);
                    waker.wake();
                });
            }
            std::task::Poll::Pending
        })
        .await;
        self.count += 1;
        String::from("later")
    }
}
`;
  static reachFiles = {
    'client/models/Greeting.rs': 'pub struct Greeting {\n    pub message: String,\n}\n',
    'client/remotes/GreetingClient.rs': `use crate::models::Greeting;

#[HTTPClient("/api")]
trait GreetingClient {
    #[GetRequest("/greeting")]
    async fn get_greeting(&self) -> BehaviorResult<Greeting>;
    #[GetRequest("/large")]
    async fn get_large(&self) -> BehaviorResult<Greeting>;
}
`,
    'client/setups/GreetingSetup.rs': `#[Setup]
struct GreetingSetup {}
impl GreetingSetup {
    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {
        Ok(BehaviorConfiguration::default())
    }
}
`,
    'client/services/RustReachService.rs': `#[Service]
pub struct RustReachService {
    #[Needs]
    greeting_client: std::sync::Arc<GreetingClient>,
    pub message: String,
}

impl RustReachService {
    pub async fn ask(&mut self) -> BehaviorResult<String> {
        self.message = self.greeting_client.get_greeting().await?.message;
        Ok(self.message.clone())
    }

    pub async fn ask_large(&mut self) -> String {
        match self.greeting_client.get_large().await {
            Ok(greeting) => greeting.message,
            Err(failure) => String::from(failure.code),
        }
    }
}
`,
  };
  static kotlinService = `@Service
class KotlinTallyService {
  var count: Int = 0
  fun increase(): Int { count += 2; return count }
  fun processId(): Int = android.os.Process.myPid()
}
`;

  static run(command, args, options = {}) {
    const result = spawnSync(command, args, { cwd: AndroidRustGate.repo, encoding: 'utf8', timeout: 900000, maxBuffer: 8 * 1024 * 1024, ...options });
    assert.equal(result.status, 0, `${command} ${args.join(' ')}: ${result.error || result.stderr || result.stdout}`);
    return result.stdout.trim();
  }

  static adbShell(...args) {
    return AndroidRustGate.run(AndroidRustGate.adb, ['shell', ...args]);
  }

  static pause(milliseconds) {
    return new Promise(resolve => setTimeout(resolve, milliseconds));
  }

  static write(project, name, contents) {
    const file = path.join(project, name);
    mkdirSync(path.dirname(file), { recursive: true });
    writeFileSync(file, contents);
  }

  static stage(project, port) {
    const write = (name, contents) => AndroidRustGate.write(project, name, contents);
    for (const [name, contents] of Object.entries(AndroidRustGate.reachFiles)) write(name, contents);
    write('client/manifest.json', JSON.stringify({ name: 'Rust Probe', start_url: '/', tac: { yon: { origin: `http://localhost:${port}` }, visibility: { check: false }, constants: { check: false }, cohesion: { check: false }, application: { id: 'dev.tachyon.rust-probe', version: '0.1.0', build: 3, window: { controls: ['title'] } } } }));
    write('client/models/RustNote.rs', 'pub struct RustNote {\n    pub text: String,\n}\n');
    write('client/services/RustCounterService.rs', AndroidRustGate.rustService);
    write('client/services/KotlinTallyService.kt', AndroidRustGate.kotlinService);
    write('client/components/rust/probe/tac.html', `<main aria-label="Rust probe">
<button on:click="rustCounter.increase()" aria-label="Increase count">Increase</button>
<output aria-label="Count">{rustCounter.count}</output>
<p>Rust counted {rustCounter.count}</p>
<p>Received {rustCounter.received}</p>
<button on:click="rustCounter.visit()">Visit</button>
<output aria-label="Visits">{rustCounter.visits}</output>
<button on:click="kotlinTally.increase()" aria-label="Increase tally">Tally</button>
<output aria-label="Tally">{kotlinTally.count}</output>
<p>Yon says {rustReach.message}</p>
</main>`);
    write('client/pages/tac.html', '<rust-probe />\n');
  }

  static binary() {
    if (process.env.TAC_BIN) return process.env.TAC_BIN;
    const run = AndroidRustGate.run;
    run('cargo', ['build', '--locked', '--bin', 'ty']);
    return path.join(JSON.parse(run('cargo', ['metadata', '--format-version', '1', '--no-deps'])).target_directory, 'debug/ty');
  }

  static checkBundle(project) {
    const host = JSON.parse(readFileSync(path.join(project, 'dist/android/tachyon.host.json'), 'utf8'));
    assert.deepEqual(host.companions, [{ service: 'kotlinTally', language: 'Kotlin' }, { service: 'rustCounter', language: 'Rust' }, { service: 'rustReach', language: 'Rust' }]);
    const entries = AndroidRustGate.run('unzip', ['-Z1', path.join(project, AndroidRustGate.apk)]).split('\n');
    for (const abi of AndroidRustGate.abis) {
      assert.ok(entries.includes(`lib/${abi}/libtachyoncompanion.so`), `The APK must carry the Rust library for ${abi}`);
    }
    assert.deepEqual(entries.filter(entry => entry.endsWith('.wasm')), [], 'Linked Rust services are not also shipped as WebAssembly');
  }

  static async launch() {
    const { adbShell, packageName, pause } = AndroidRustGate;
    adbShell('am', 'force-stop', packageName);
    adbShell('am', 'start', '-W', '-n', `${packageName}/.MainActivity`);
    let pid = '';
    for (let attempt = 0; attempt < 60; attempt++) {
      pid = adbShell('pidof', packageName);
      if (pid && adbShell('cat', '/proc/net/unix').includes(`webview_devtools_remote_${pid}`)) break;
      await pause(500);
    }
    assert.match(pid, /^\d+$/, 'The Rust probe app must start');
    return pid;
  }

  static async connect(pid) {
    const port = AndroidRustGate.run(AndroidRustGate.adb, ['forward', 'tcp:0', `localabstract:webview_devtools_remote_${pid}`]);
    const browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`, { noDefaults: true });
    const page = browser.contexts()[0].pages()[0];
    assert.ok(page, 'Android WebView exposes a live document');
    page.on('pageerror', error => console.error('Android page error:', error.message));
    await page.getByRole('button', { name: 'Increase count' }).waitFor();
    return { port, browser, page };
  }

  static async disconnect(connection) {
    if (!connection) return;
    await connection.browser.close();
    AndroidRustGate.run(AndroidRustGate.adb, ['forward', '--remove', `tcp:${connection.port}`]);
  }

  static applicationWindowTitle() {
    const { adb, adbShell, repo } = AndroidRustGate;
    const dump = spawnSync(adb, ['shell', 'uiautomator', 'dump', '--windows', '/sdcard/tachyon-windows.xml'], { cwd: repo, encoding: 'utf8', timeout: 60000 });
    if (!`${dump.stdout}${dump.stderr}`.includes('dumped to')) return '';
    return adbShell('cat', '/sdcard/tachyon-windows.xml').match(/<window [^>]*title="([^"]*)"[^>]*type="TYPE_APPLICATION"/)?.[1] ?? '';
  }

  static async checkCalls(page, pid) {
    const service = name => `__tachyonTac.service('${name}')`;
    await page.getByRole('button', { name: 'Increase count' }).click();
    await page.getByRole('status', { name: 'Count' }).filter({ hasText: /^21$/ }).waitFor();
    assert.equal(await page.evaluate(`${service('rustCounter')}.processId()`), Number(pid), 'Rust must run inside the Android app process');
    assert.equal(await page.evaluate(`${service('kotlinTally')}.processId()`), Number(pid), 'Kotlin still links next to Rust');
    await page.getByRole('button', { name: 'Increase tally' }).click();
    await page.getByRole('status', { name: 'Tally' }).filter({ hasText: /^2$/ }).waitFor();
    assert.equal(await page.evaluate(`${service('rustCounter')}.later()`), 'later', 'An awaited Rust method resumes on the host');
    await page.getByRole('button', { name: 'Increase count' }).click();
    await page.getByRole('status', { name: 'Count' }).filter({ hasText: /^43$/ }).waitFor();
  }

  static async checkEvent(page) {
    await page.evaluate(async () => {
      const receive = globalThis.__tachyonCompanionPublish;
      globalThis.__tachyonCompanionPublish = event => { globalThis.rustEvidence = event; receive(event); };
      await __tachyonTac.service('rustCounter').announce();
    });
    await page.waitForFunction(() => globalThis.rustEvidence?.name === 'RustNote');
    assert.deepEqual(await page.evaluate(() => globalThis.rustEvidence.value), { text: 'Rust' });
    assert.equal(await page.evaluate(() => __tachyonTac.service('rustCounter').received), 'Rust', 'The Rust listener receives its own event in process');
    await page.getByRole('button', { name: 'Increase count' }).click();
    await page.getByText('Received Rust', { exact: true }).waitFor();
  }

  static async checkWindowControls(page) {
    assert.equal(await page.evaluate(`__tachyonTac.service('rustCounter').grants()`), 'title=true close=false');
    assert.equal(await page.evaluate(`__tachyonTac.service('rustCounter').rename()`), '', 'A granted control reports no problem');
    for (let attempt = 0; attempt < 40 && AndroidRustGate.applicationWindowTitle() !== 'Rust titled'; attempt++) await AndroidRustGate.pause(250);
    assert.equal(AndroidRustGate.applicationWindowTitle(), 'Rust titled', 'Rust sets the Android window title through the host');
  }

  static async checkStoredField(page) {
    const visits = page.getByRole('status', { name: 'Visits' });
    await page.getByRole('button', { name: 'Visit' }).click();
    await visits.filter({ hasText: /^1$/ }).waitFor();
    await page.reload();
    await page.getByRole('button', { name: 'Increase count' }).waitFor();
    await visits.filter({ hasText: /^1$/ }).waitFor({ timeout: 5000 });
    await page.getByRole('button', { name: 'Visit' }).click();
    await visits.filter({ hasText: /^2$/ }).waitFor();
  }

  static yon() {
    const requests = [];
    const server = createServer((request, response) => {
      requests.push(`${request.method} ${request.url}`);
      const large = request.url === '/api/large';
      const message = large ? 'x'.repeat(1_100_000) : 'Hello from Yon to Rust';
      response.writeHead(200, { 'content-type': 'application/json' });
      response.end(JSON.stringify({ message }));
    });
    return new Promise(resolve => server.listen(0, '127.0.0.1', () => resolve({ server, requests, port: server.address().port })));
  }

  static async checkYon(page, yon) {
    const reach = `__tachyonTac.service('rustReach')`;
    assert.equal(await page.evaluate(`${reach}.ask()`), 'Hello from Yon to Rust', 'A linked Rust HTTP client reaches tac.yon.origin');
    assert.equal(await page.evaluate(`${reach}.askLarge()`), 'HTTP_LIMIT', 'A Yon answer over the size limit is refused');
    assert.deepEqual(yon.requests, ['GET /api/greeting', 'GET /api/large']);
  }

  static apksigner() {
    const tools = path.join(AndroidRustGate.sdk, 'build-tools');
    const latest = readdirSync(tools).sort((left, right) => left.localeCompare(right, undefined, { numeric: true })).at(-1);
    return path.join(tools, latest, process.platform === 'win32' ? 'apksigner.bat' : 'apksigner');
  }

  static buildRelease(binary, project) {
    const { run, release, abis, apksigner } = AndroidRustGate;
    const keys = mkdtempSync(path.join(tmpdir(), 'tachyon-android-rust-keys-'));
    const keystore = path.join(keys, 'upload.p12');
    run('keytool', ['-genkeypair', '-keystore', keystore, '-storetype', 'PKCS12', '-storepass', release.password, '-keypass', release.password, '-alias', release.alias, '-keyalg', 'RSA', '-keysize', '2048', '-validity', '2', '-dname', release.signer]);
    const env = { ...process.env, TAC_ANDROID_KEYSTORE: keystore, TAC_ANDROID_KEYSTORE_PASSWORD: release.password, TAC_ANDROID_KEYSTORE_ALIAS: release.alias };
    try {
      const built = run(binary, ['bundle', project, '--target', 'android', '--release', '--out-dir', 'dist-release'], { env });
      assert.ok(!built.includes(release.password), 'A release build never prints the keystore password');
    } finally {
      rmSync(keys, { recursive: true, force: true });
    }
    const apk = path.join(project, release.bundle, 'RustProbe.apk');
    const aab = path.join(project, release.bundle, 'RustProbe.aab');
    assert.match(run(apksigner(), ['verify', '--print-certs', apk]), /CN=Tachyon Release Gate/, 'The release APK is signed with the upload key');
    assert.match(run('jarsigner', ['-verify', '-verbose', '-certs', aab]), /CN="?Tachyon Release Gate/, 'The App Bundle is signed with the upload key');
    const apkEntries = run('unzip', ['-Z1', apk]).split('\n');
    const aabEntries = run('unzip', ['-Z1', aab]).split('\n');
    for (const abi of abis) {
      assert.ok(apkEntries.includes(`lib/${abi}/libtachyoncompanion.so`), `The release APK carries the Rust library for ${abi}`);
      assert.ok(aabEntries.includes(`base/lib/${abi}/libtachyoncompanion.so`), `The App Bundle carries the Rust library for ${abi}`);
    }
    return apk;
  }

  static screen() {
    AndroidRustGate.adbShell('uiautomator', 'dump', '/sdcard/tachyon-rust-release.xml');
    return AndroidRustGate.adbShell('cat', '/sdcard/tachyon-rust-release.xml');
  }

  static async tapOnScreen(label) {
    const { screen, adbShell, pause } = AndroidRustGate;
    let bounds;
    for (let attempt = 0; attempt < 120 && !bounds; attempt++) {
      bounds = screen().match(new RegExp(`<node [^>]*(?:text|content-desc)="${label}"[^>]*bounds="\\[(\\d+),(\\d+)\\]\\[(\\d+),(\\d+)\\]"`));
      if (!bounds) await pause(500);
    }
    assert.ok(bounds, `The release app shows ${label}`);
    const [, left, top, right, bottom] = bounds.map(Number);
    adbShell('input', 'tap', String(Math.round((left + right) / 2)), String(Math.round((top + bottom) / 2)));
  }

  static async describeRelease(pid) {
    const connection = await AndroidRustGate.connect(pid).catch(error => ({ error }));
    if (connection.error) { console.error('Android release page:', connection.error.message); return; }
    console.error('Android release page:', await connection.page.locator('body').innerText());
    await AndroidRustGate.disconnect(connection);
  }

  static async checkRelease(binary, project) {
    const { run, adb, adbShell, packageName, buildRelease, tapOnScreen, screen, pause } = AndroidRustGate;
    const apk = buildRelease(binary, project);
    spawnSync(adb, ['uninstall', packageName], { encoding: 'utf8', timeout: 60000 });
    try {
      run(adb, ['install', apk]);
      assert.doesNotMatch(adbShell('dumpsys', 'package', packageName).match(/pkgFlags=\[[^\]]*\]/)?.[0] ?? '', /DEBUGGABLE/, 'The release APK is not debuggable');
      adbShell('am', 'start', '-W', '-n', `${packageName}/.MainActivity`);
      let pid = '';
      for (let attempt = 0; attempt < 40 && !/^\d+$/.test(pid); attempt++) {
        await pause(500);
        pid = spawnSync(adb, ['shell', 'pidof', packageName], { encoding: 'utf8' }).stdout.trim();
      }
      assert.match(pid, /^\d+$/, 'The release APK launches');
      await tapOnScreen('Increase count');
      let counted = false;
      for (let attempt = 0; attempt < 40 && !counted; attempt++) {
        counted = /<node [^>]*text="Rust counted 21"/.test(screen());
        if (!counted) await pause(500);
      }
      const debuggable = adbShell('getprop', 'ro.debuggable') === '1';
      if (!counted && debuggable) await AndroidRustGate.describeRelease(pid);
      assert.ok(counted, `A tap in the release app reaches the linked Rust service, which counts 21: ${screen().match(/<node [^>]*(?:text|content-desc)="[^"]+"/g)?.join(' ')}`);
      if (!debuggable) assert.ok(!adbShell('cat', '/proc/net/unix').includes(`webview_devtools_remote_${pid}`), 'A release build does not open WebView debugging');
      const abi = adbShell('getprop', 'ro.product.cpu.abi');
      console.log(`PASS: Android signed release: the APK and App Bundle carry the Rust library for ${AndroidRustGate.abis.join(' and ')}; the release APK installed on ${abi} (ro.debuggable=${debuggable ? 1 : 0}) and a tap reached the linked Rust service in process ${pid}`);
    } finally {
      spawnSync(adb, ['shell', 'am', 'force-stop', packageName], { encoding: 'utf8', timeout: 60000 });
      spawnSync(adb, ['uninstall', packageName], { encoding: 'utf8', timeout: 60000 });
    }
  }

  static async main() {
    assert.ok(AndroidRustGate.sdk, 'ANDROID_HOME or ANDROID_SDK_ROOT is required');
    const project = mkdtempSync(path.join(tmpdir(), 'tachyon-android-rust-'));
    const yon = await AndroidRustGate.yon();
    let connection;
    let reversed = false;
    try {
      AndroidRustGate.stage(project, yon.port);
      const binary = AndroidRustGate.binary();
      AndroidRustGate.run(binary, ['build', project, '--target', 'android']);
      AndroidRustGate.checkBundle(project);
      AndroidRustGate.run(AndroidRustGate.adb, ['wait-for-device']);
      AndroidRustGate.run(AndroidRustGate.adb, ['reverse', `tcp:${yon.port}`, `tcp:${yon.port}`]);
      reversed = true;
      AndroidRustGate.run(AndroidRustGate.adb, ['install', '-r', path.join(project, AndroidRustGate.apk)]);
      AndroidRustGate.adbShell('pm', 'clear', AndroidRustGate.packageName);
      let pid = await AndroidRustGate.launch();
      connection = await AndroidRustGate.connect(pid);
      await AndroidRustGate.checkCalls(connection.page, pid);
      await AndroidRustGate.checkEvent(connection.page);
      await AndroidRustGate.checkWindowControls(connection.page);
      await AndroidRustGate.checkYon(connection.page, yon);
      await AndroidRustGate.checkStoredField(connection.page);
      await connection.page.evaluate(`__tachyonTac.service('rustCounter').remember()`);
      await AndroidRustGate.disconnect(connection);
      connection = undefined;
      pid = await AndroidRustGate.launch();
      connection = await AndroidRustGate.connect(pid);
      assert.equal(await connection.page.evaluate(`__tachyonTac.service('rustCounter').recall()`), 'kept', 'tac_set_local keeps a value in the app files across a relaunch');
      console.log(`PASS: Android linked Rust: ${AndroidRustGate.abis.join(' and ')} libraries, calls, an awaited method, an event, window controls, a Yon HTTP client, @LocalState and tac_set_local in process ${pid}, next to a Kotlin service`);
      await AndroidRustGate.disconnect(connection);
      connection = undefined;
      await AndroidRustGate.checkRelease(binary, project);
    } finally {
      await AndroidRustGate.disconnect(connection);
      AndroidRustGate.adbShell('am', 'force-stop', AndroidRustGate.packageName);
      if (reversed) AndroidRustGate.run(AndroidRustGate.adb, ['reverse', '--remove', `tcp:${yon.port}`]);
      yon.server.close();
      rmSync(project, { recursive: true, force: true });
    }
  }
}

await AndroidRustGate.main();
