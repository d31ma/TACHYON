#!/usr/bin/env node

import { CargoArtifacts } from './cargo-artifacts.mjs'
import { chromium, firefox, webkit } from 'playwright'
import { createServer } from 'node:http'
import { spawnSync } from 'node:child_process'
import { createReadStream, existsSync, mkdtempSync, mkdirSync, rmSync, statSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import process from 'node:process'

class WebApiGate {
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('release/ty')
  static mimeTypes = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.wasm': 'application/wasm' }
  static uuidPattern = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/
  static withJava = Boolean(process.env.TAC_TEAVM_HOME) && existsSync(process.env.TAC_TEAVM_HOME)
  static withFsharp = spawnSync('dotnet', ['--version'], { encoding: 'utf8' }).status === 0
  static languages = ['js', 'rust', 'go', 'swift', ...(WebApiGate.withJava ? ['java'] : []), ...(WebApiGate.withFsharp ? ['fs'] : [])]
  static everyEngineLanguages = WebApiGate.withFsharp ? ['fs'] : []
  static otherEngines = [firefox, webkit]
  static permissions = ['clipboard-read', 'clipboard-write', 'geolocation', 'notifications']
  static position = { latitude: 48.85, longitude: 2.35 }
  static settleTimeout = 5_000
  static settleStep = 50
  static activationLifespan = 6_000
  static napMs = 20
  static consoleLines = []

  static files = {
    'client/manifest.json': '{"tac":{"browserApis":["clipboard","share","notifications","geolocation"],"names":{"words":["js","fs"]},"visibility":{"check":false}}}\n',
    'client/pages/tac.html': '<stamp-index />\n',
    'client/components/stamp/index/tac.html': '<main><h1>Browser APIs</h1></main>\n',
    ...Object.fromEntries(WebApiGate.languages.flatMap((language) => [
      [`client/pages/${language}/tac.html`, `<stamp-${language} />\n`],
      [`client/components/stamp/${language}/tac.html`, `<main><p id="${language}">{${language}Stamp.label}</p>
<button id="${language}-copy" on:click="${language}Stamp.copy('from ${language}')">Copy</button>
<button id="${language}-paste" on:click="${language}Stamp.paste()">Paste</button>
<button id="${language}-ask" on:click="${language}Stamp.ask()">Ask</button>
<button id="${language}-tell" on:click="${language}Stamp.tell()">Share</button></main>\n`],
    ])),
    'client/services/JsStampService.js': `@Service
export default class JsStampService {
  @Needs
  clock;
  @Needs
  secureRandom;
  @Needs
  preferences;
  @Needs
  clipboard;
  @Needs
  share;
  @Needs
  notifications;
  @Needs
  geolocation;
  label = 'js';
  online = false;
  scheme = '';
  pasted = '';
  permission = '';
  latitude = 0;
  failure = '';
  shared = false;

  now() {
    return this.clock.now();
  }

  zone() {
    return this.clock.timeZone();
  }

  token() {
    return this.secureRandom.uuid();
  }

  count() {
    return this.secureRandom.bytes(16).length;
  }

  motion() {
    return this.preferences.motion();
  }

  locale() {
    return this.preferences.language();
  }

  copy(click, text) {
    this.clipboard.writeText(text);
  }

  attempt() {
    return this.#refusal();
  }

  #refusal() {
    try {
      this.clipboard.writeText('without a click');
      return '';
    } catch (error) {
      return error.message;
    }
  }

  paste() {
    this.clipboard.readText();
  }

  ask() {
    this.notifications.requestPermission();
  }

  tell() {
    this.share.open('Tachyon', 'Shared from a test', 'https://example.com/');
  }

  locate() {
    this.geolocation.currentPosition();
  }

  /** @param {ConnectivityChanged} changed */
  @OnEvent({"replay":true})
  hearConnectivity(changed) {
    this.online = changed.online;
  }

  /** @param {PreferencesChanged} changed */
  @OnEvent({"replay":true})
  hearPreferences(changed) {
    this.scheme = changed.scheme;
  }

  /** @param {ClipboardTextRead} read */
  @OnEvent
  hearClipboard(read) {
    this.pasted = read.text;
  }

  /** @param {NotificationPermissionChanged} changed */
  @OnEvent
  hearPermission(changed) {
    this.permission = changed.permission;
  }

  /** @param {PositionFound} found */
  @OnEvent
  hearPosition(found) {
    this.latitude = found.latitude;
  }

  /** @param {ShareFinished} finished */
  @OnEvent
  hearShare(finished) {
    this.shared = Object.keys(finished).length === 0;
  }

  /** @param {BrowserCallFailed} failed */
  @OnEvent
  hearFailure(failed) {
    this.failure = failed.message;
  }
}
`,
    'client/services/RustStampService.rs': `#[Service]
#[derive(Default)]
struct RustStampService {
    clock: Clock,
    secure_random: SecureRandom,
    preferences: Preferences,
    clipboard: Clipboard,
    share: Share,
    notifications: Notifications,
    geolocation: Geolocation,
    label: String,
    online: bool,
    scheme: String,
    pasted: String,
    permission: String,
    latitude: f64,
    failure: String,
    shared: bool,
}

impl RustStampService {
    fn now(&self) -> f64 {
        self.clock.now_ms()
    }

    fn zone(&self) -> String {
        self.clock.time_zone().unwrap_or_default()
    }

    fn token(&self) -> String {
        self.secure_random.uuid().unwrap_or_default()
    }

    fn count(&self) -> i64 {
        self.secure_random.bytes(16).map_or(-1, |bytes| bytes.len() as i64)
    }

    fn motion(&self) -> String {
        self.preferences.motion().unwrap_or_default()
    }

    fn locale(&self) -> String {
        self.preferences.language().unwrap_or_default()
    }

    fn copy(&self, text: &str) {
        let _ = self.clipboard.write_text(text);
    }

    fn attempt(&self) -> String {
        self.clipboard.write_text("without a click").err().unwrap_or_default()
    }

    fn paste(&self) {
        let _ = self.clipboard.read_text();
    }

    fn ask(&self) {
        let _ = self.notifications.request_permission();
    }

    fn tell(&self) {
        let _ = self.share.open("Tachyon", "Shared from a test", "https://example.com/");
    }

    fn locate(&self) {
        let _ = self.geolocation.current_position();
    }

    #[OnEvent({"replay":true})]
    fn hear_connectivity(&mut self, changed: ConnectivityChanged) {
        self.online = changed.online;
    }

    #[OnEvent({"replay":true})]
    fn hear_preferences(&mut self, changed: PreferencesChanged) {
        self.scheme = changed.scheme;
    }

    #[OnEvent]
    fn hear_clipboard(&mut self, read: ClipboardTextRead) {
        self.pasted = read.text;
    }

    #[OnEvent]
    fn hear_permission(&mut self, changed: NotificationPermissionChanged) {
        self.permission = changed.permission;
    }

    #[OnEvent]
    fn hear_position(&mut self, found: PositionFound) {
        self.latitude = found.latitude;
    }

    #[OnEvent]
    fn hear_share(&mut self, finished: ShareFinished) {
        self.shared = finished == ShareFinished {};
    }

    #[OnEvent]
    fn hear_failure(&mut self, failed: BrowserCallFailed) {
        self.failure = failed.message;
    }
}
`,
    'client/services/GoStampService.go': `// @Service
type GoStampService struct {
	Label      string
	Online     bool
	Scheme     string
	Pasted     string
	Permission string
	Latitude   float64
	Failure    string
	Shared     bool
	// @Needs
	clock Clock
	// @Needs
	secureRandom SecureRandom
	// @Needs
	preferences Preferences
	// @Needs
	clipboard Clipboard
	// @Needs
	share Share
	// @Needs
	notifications Notifications
	// @Needs
	geolocation Geolocation
}

func (stamp *GoStampService) Now() float64 {
	value, _ := stamp.clock.NowMs()
	return value
}

func (stamp *GoStampService) Zone() string {
	value, _ := stamp.clock.TimeZone()
	return value
}

func (stamp *GoStampService) Token() string {
	value, _ := stamp.secureRandom.UUID()
	return value
}

func (stamp *GoStampService) Count() int {
	bytes, _ := stamp.secureRandom.Bytes(16)
	return len(bytes)
}

func (stamp *GoStampService) Motion() string {
	value, _ := stamp.preferences.Motion()
	return value
}

func (stamp *GoStampService) Locale() string {
	value, _ := stamp.preferences.Language()
	return value
}

func (stamp *GoStampService) Copy(text string) {
	_ = stamp.clipboard.WriteText(text)
}

func (stamp *GoStampService) Attempt() string {
	return stamp.refusal(stamp.clipboard.WriteText("without a click"))
}

func (stamp *GoStampService) refusal(problem error) string {
	if problem == nil {
		return ""
	}
	return problem.Error()
}

func (stamp *GoStampService) Paste() {
	_ = stamp.clipboard.ReadText()
}

func (stamp *GoStampService) Ask() {
	_ = stamp.notifications.RequestPermission()
}

func (stamp *GoStampService) Tell() {
	_ = stamp.share.Open("Tachyon", "Shared from a test", "https://example.com/")
}

func (stamp *GoStampService) Locate() {
	_ = stamp.geolocation.CurrentPosition()
}

// @OnEvent({"replay":true})
func (stamp *GoStampService) HearConnectivity(changed ConnectivityChanged) {
	stamp.Online = changed.Online
}

// @OnEvent({"replay":true})
func (stamp *GoStampService) HearPreferences(changed PreferencesChanged) {
	stamp.Scheme = changed.Scheme
}

// @OnEvent
func (stamp *GoStampService) HearClipboard(read ClipboardTextRead) {
	stamp.Pasted = read.Text
}

// @OnEvent
func (stamp *GoStampService) HearPermission(changed NotificationPermissionChanged) {
	stamp.Permission = changed.Permission
}

// @OnEvent
func (stamp *GoStampService) HearPosition(found PositionFound) {
	stamp.Latitude = found.Latitude
}

// @OnEvent
func (stamp *GoStampService) HearShare(finished ShareFinished) {
	stamp.Shared = finished == ShareFinished{}
}

// @OnEvent
func (stamp *GoStampService) HearFailure(failed BrowserCallFailed) {
	stamp.Failure = failed.Message
}
`,
    'client/services/SwiftStampService.swift': `@Service
final class SwiftStampService {
    private let clock = Clock()
    private let secureRandom = SecureRandom()
    private let preferences = Preferences()
    private let clipboard = Clipboard()
    private let share = Share()
    private let notifications = Notifications()
    private let geolocation = Geolocation()
    var label: String = "swift"
    var online: Bool = false
    var scheme: String = ""
    var pasted: String = ""
    var permission: String = ""
    var latitude: Double = 0
    var failure: String = ""
    var shared: Bool = false

    func now() -> Double {
        clock.nowMs()
    }

    func zone() throws -> String {
        try clock.timeZone()
    }

    func token() throws -> String {
        try secureRandom.uuid()
    }

    func count() throws -> Int {
        try secureRandom.bytes(16).count
    }

    func motion() throws -> String {
        try preferences.motion()
    }

    func locale() throws -> String {
        try preferences.language()
    }

    func copy(_ text: String) throws {
        try clipboard.writeText(text)
    }

    func attempt() -> String {
        refusal { try clipboard.writeText("without a click") }
    }

    func paste() throws {
        try clipboard.readText()
    }

    func ask() throws {
        try notifications.requestPermission()
    }

    func tell() throws {
        try share.open(title: "Tachyon", text: "Shared from a test", url: "https://example.com/")
    }

    func locate() throws {
        try geolocation.currentPosition()
    }

    @OnEvent({"replay":true})
    func hearConnectivity(_ changed: ConnectivityChanged) {
        online = changed.online
    }

    @OnEvent({"replay":true})
    func hearPreferences(_ changed: PreferencesChanged) {
        scheme = changed.scheme
    }

    @OnEvent
    func hearClipboard(_ read: ClipboardTextRead) {
        pasted = read.text
    }

    @OnEvent
    func hearPermission(_ changed: NotificationPermissionChanged) {
        permission = changed.permission
    }

    @OnEvent
    func hearPosition(_ found: PositionFound) {
        latitude = found.latitude
    }

    @OnEvent
    func hearShare(_ finished: ShareFinished) {
        shared = true
    }

    @OnEvent
    func hearFailure(_ failed: BrowserCallFailed) {
        failure = failed.message
    }

    private func refusal(_ call: () throws -> Void) -> String {
        do {
            try call()
            return ""
        } catch {
            return String(describing: error)
        }
    }
}
`,
    ...(WebApiGate.withFsharp ? { 'client/services/FsStampService.fs': `[<Service>]
type FsStampService() =
    let clock = Clock()
    let secureRandom = SecureRandom()
    let preferences = Preferences()
    let clipboard = Clipboard()
    let share = Share()
    let notifications = Notifications()
    let geolocation = Geolocation()
    member val Label : string = "fs" with get, set
    member val Online : bool = false with get, set
    member val Scheme : string = "" with get, set
    member val Pasted : string = "" with get, set
    member val Permission : string = "" with get, set
    member val Latitude : float = 0.0 with get, set
    member val Failure : string = "" with get, set
    member val Shared : bool = false with get, set

    member this.Now() : float = clock.NowMs()

    member this.Zone() : string = clock.TimeZone()

    member this.Token() : string = secureRandom.Uuid()

    member this.Count() : int = secureRandom.Bytes(16).Length

    member this.Motion() : string = preferences.Motion()

    member this.Locale() : string = preferences.Language()

    member this.Copy(text: string) : unit = clipboard.WriteText(text)

    member this.Attempt() : string = this.Refusal()

    member this.Paste() : unit = clipboard.ReadText()

    member this.Ask() : unit = notifications.RequestPermission()

    member this.Tell() : unit = share.Open("Tachyon", "Shared from a test", "https://example.com/")

    member this.Locate() : unit = geolocation.CurrentPosition()

    member this.Forget() =
        this.Pasted <- ""

    member this.Year() : int = System.DateTime.UtcNow.Year

    member this.Roll() : int = System.Random().Next(1, 7)

    member this.Say() : unit = printfn "fs prints to the console"

    member this.Nap() : unit = System.Threading.Thread.Sleep(20)

    [<OnEvent({"replay":true})>]
    member this.HearConnectivity(changed: ConnectivityChanged) : unit =
        this.Online <- changed.Online

    [<OnEvent({"replay":true})>]
    member this.HearPreferences(changed: PreferencesChanged) : unit =
        this.Scheme <- changed.Scheme

    [<OnEvent>]
    member this.HearClipboard(read: ClipboardTextRead) : unit =
        this.Pasted <- read.Text

    [<OnEvent>]
    member this.HearPermission(changed: NotificationPermissionChanged) : unit =
        this.Permission <- changed.Permission

    [<OnEvent>]
    member this.HearPosition(found: PositionFound) : unit =
        this.Latitude <- found.Latitude

    [<OnEvent>]
    member this.HearShare(finished: ShareFinished) : unit =
        this.Shared <- not (isNull (box finished))

    [<OnEvent>]
    member this.HearFailure(failed: BrowserCallFailed) : unit =
        this.Failure <- failed.Message

    member private this.Refusal() : string =
        try
            clipboard.WriteText("without a click")
            ""
        with :? System.InvalidOperationException as refused -> refused.Message
` } : {}),
    ...(WebApiGate.withJava ? { 'client/services/JavaStampService.java': `@Service
class JavaStampService {
    private final Clock clock = new Clock();
    private final SecureRandom secureRandom = new SecureRandom();
    private final Preferences preferences = new Preferences();
    private final Clipboard clipboard = new Clipboard();
    private final Share share = new Share();
    private final Notifications notifications = new Notifications();
    private final Geolocation geolocation = new Geolocation();
    String label = "java";
    boolean online = false;
    String scheme = "";
    String pasted = "";
    String permission = "";
    double latitude = 0;
    String failure = "";
    boolean shared = false;

    double now() {
        return clock.nowMs();
    }

    String zone() {
        return clock.timeZone();
    }

    String token() {
        return secureRandom.uuid();
    }

    int count() {
        return secureRandom.bytes(16).length;
    }

    String motion() {
        return preferences.motion();
    }

    String locale() {
        return preferences.language();
    }

    void copy(String text) {
        clipboard.writeText(text);
    }

    String attempt() {
        return refusal(() -> clipboard.writeText("without a click"));
    }

    void paste() {
        clipboard.readText();
    }

    void ask() {
        notifications.requestPermission();
    }

    void tell() {
        share.open("Tachyon", "Shared from a test", "https://example.com/");
    }

    void locate() {
        geolocation.currentPosition();
    }

    @OnEvent({"replay":true})
    void hearConnectivity(ConnectivityChanged changed) {
        online = changed.online();
    }

    @OnEvent({"replay":true})
    void hearPreferences(PreferencesChanged changed) {
        scheme = changed.scheme();
    }

    @OnEvent
    void hearClipboard(ClipboardTextRead read) {
        pasted = read.text();
    }

    @OnEvent
    void hearPermission(NotificationPermissionChanged changed) {
        permission = changed.permission();
    }

    @OnEvent
    void hearPosition(PositionFound found) {
        latitude = found.latitude();
    }

    @OnEvent
    void hearShare(ShareFinished finished) {
        shared = true;
    }

    @OnEvent
    void hearFailure(BrowserCallFailed failed) {
        failure = failed.message();
    }

    private String refusal(Runnable call) {
        try {
            call.run();
            return "";
        } catch (IllegalStateException refused) {
            return refused.getMessage();
        }
    }
}
` } : {}),
  }

  static write(project) {
    for (const [relative, contents] of Object.entries(WebApiGate.files)) {
      const file = path.join(project, relative)
      mkdirSync(path.dirname(file), { recursive: true })
      writeFileSync(file, contents)
    }
  }

  static build(project) {
    const built = spawnSync(WebApiGate.binary, ['build', project], {
      encoding: 'utf8', timeout: 300_000, killSignal: 'SIGKILL', maxBuffer: 4 * 1024 * 1024,
    })
    if (built.status !== 0) throw new Error(`web API fixture build failed: ${built.error?.message || built.stderr || built.stdout}`)
  }

  static handler(project) {
    const root = path.join(project, 'dist')
    return (request, response) => {
      const url = new URL(request.url, 'http://localhost')
      let file = path.join(root, decodeURIComponent(url.pathname))
      if (existsSync(file) && statSync(file).isDirectory()) file = path.join(file, 'index.html')
      if (!existsSync(file)) {
        response.writeHead(404).end()
        return
      }
      response.writeHead(200, { 'content-type': WebApiGate.mimeTypes[path.extname(file)] ?? 'application/octet-stream' })
      createReadStream(file).pipe(response)
    }
  }

  static expect(actual, wanted, label) {
    if (actual !== wanted) throw new Error(`${label}: expected ${JSON.stringify(wanted)}, got ${JSON.stringify(actual)}`)
    console.log(`ok - ${label}`)
  }

  static async check(page, language, label = language) {
    const report = await page.evaluate((name) => {
      const service = __tachyonTac.service(`${name}Stamp`)
      const before = Date.now()
      const values = { now: service.now(), zone: service.zone(), token: service.token(), count: service.count() }
      return { ...values, before, after: Date.now(), browserZone: Intl.DateTimeFormat().resolvedOptions().timeZone }
    }, language)
    WebApiGate.expect(report.now >= report.before && report.now <= report.after, true, `${label}: Clock gives the time of the browser`)
    WebApiGate.expect(report.zone, report.browserZone, `${label}: Clock gives the time zone of the browser`)
    WebApiGate.expect(WebApiGate.uuidPattern.test(report.token), true, `${label}: SecureRandom gives a version 4 UUID`)
    WebApiGate.expect(report.count, 16, `${label}: SecureRandom gives the bytes it was asked for`)
  }

  static refusals(page, language) {
    return page.evaluate(([name, lifespan]) => new Promise((resolve) => {
      setTimeout(() => {
        resolve({ active: navigator.userActivation.isActive, answer: __tachyonTac.service(`${name}Stamp`).attempt() })
      }, lifespan)
    }), [language, WebApiGate.activationLifespan])
  }

  static async open(page, origin, language) {
    await page.goto(`${origin}/${language}/`, { waitUntil: 'networkidle' })
  }

  static read(page, language, member) {
    return page.evaluate(([name, key]) => {
      const service = __tachyonTac.service(`${name}Stamp`)
      return typeof service[key] === 'function' ? service[key]() : service[key]
    }, [language, member])
  }

  static async settled(page, language, member, wanted) {
    const deadline = Date.now() + WebApiGate.settleTimeout
    let value = await WebApiGate.read(page, language, member)
    while (value !== wanted && Date.now() < deadline) {
      await page.waitForTimeout(WebApiGate.settleStep)
      value = await WebApiGate.read(page, language, member)
    }
    return value
  }

  static async checkPreferences(page, context, language) {
    WebApiGate.expect(await WebApiGate.read(page, language, 'motion'), 'reduce', `${language}: Preferences gives the motion setting`)
    WebApiGate.expect(await WebApiGate.read(page, language, 'locale'), 'fr-FR', `${language}: Preferences gives the language`)
    WebApiGate.expect(await WebApiGate.settled(page, language, 'scheme', 'dark'), 'dark', `${language}: a replay listener hears the current color scheme`)
    await page.emulateMedia({ colorScheme: 'light' })
    WebApiGate.expect(await WebApiGate.settled(page, language, 'scheme', 'light'), 'light', `${language}: PreferencesChanged reaches the listener`)
    await page.emulateMedia({ colorScheme: 'dark' })
    await context.setOffline(true)
    WebApiGate.expect(await WebApiGate.settled(page, language, 'online', false), false, `${language}: ConnectivityChanged reaches the listener when the browser goes offline`)
    await context.setOffline(false)
    WebApiGate.expect(await WebApiGate.settled(page, language, 'online', true), true, `${language}: ConnectivityChanged reaches the listener when the browser is online again`)
  }

  static async checkPrompts(page, language) {
    await page.click(`#${language}-copy`)
    const copied = await page.evaluate(() => navigator.clipboard.readText())
    WebApiGate.expect(copied, `from ${language}`, `${language}: Clipboard writes text from a click, so the bridge keeps the user action`)
    await page.click(`#${language}-paste`)
    WebApiGate.expect(await WebApiGate.settled(page, language, 'pasted', `from ${language}`), `from ${language}`, `${language}: ClipboardTextRead reaches the calling service`)
    await page.click(`#${language}-ask`)
    WebApiGate.expect(await WebApiGate.settled(page, language, 'permission', 'granted'), 'granted', `${language}: NotificationPermissionChanged reaches the calling service`)
    await page.evaluate((name) => __tachyonTac.service(`${name}Stamp`).locate(), language)
    WebApiGate.expect(await WebApiGate.settled(page, language, 'latitude', WebApiGate.position.latitude), WebApiGate.position.latitude, `${language}: PositionFound reaches the calling service`)
    await page.click(`#${language}-tell`)
    const deadline = Date.now() + WebApiGate.settleTimeout
    let answered = false
    while (!answered && Date.now() < deadline) {
      answered = (await WebApiGate.read(page, language, 'shared')) === true || (await WebApiGate.read(page, language, 'failure')) !== ''
      if (!answered) await page.waitForTimeout(WebApiGate.settleStep)
    }
    WebApiGate.expect(answered, true, `${language}: Share answers the calling service with ShareFinished or BrowserCallFailed`)
  }

  static async checkUnitMethod(page, language, label = language) {
    const answer = await page.evaluate((name) => __tachyonTac.service(`${name}Stamp`).forget() ?? null, language)
    WebApiGate.expect(answer, null, `${label}: a method that returns nothing answers nothing`)
    WebApiGate.expect(await WebApiGate.settled(page, language, 'pasted', ''), '', `${label}: a method that returns nothing still runs`)
  }

  static async checkOwnLanguage(page, language) {
    await WebApiGate.checkUnitMethod(page, language)
    await WebApiGate.checkOwnRuntime(page, language)
  }

  static async checkOwnRuntime(page, language) {
    const report = await page.evaluate((name) => {
      const service = __tachyonTac.service(`${name}Stamp`)
      service.say()
      const before = Date.now()
      service.nap()
      return { year: service.year(), roll: service.roll(), napped: Date.now() - before, browserYear: new Date().getUTCFullYear() }
    }, language)
    WebApiGate.expect(report.year, report.browserYear, `${language}: the language's own clock gives the time`)
    WebApiGate.expect(report.roll >= 1 && report.roll <= 6, true, `${language}: the language's own random numbers work`)
    WebApiGate.expect(report.napped >= WebApiGate.napMs, true, `${language}: a sleep makes the page wait`)
    const printed = `${language} prints to the console`
    const deadline = Date.now() + WebApiGate.settleTimeout
    while (!WebApiGate.consoleLines.includes(printed) && Date.now() < deadline) await page.waitForTimeout(WebApiGate.settleStep)
    WebApiGate.expect(WebApiGate.consoleLines.includes(printed), true, `${language}: console output reaches the browser console`)
  }

  static async checkEngine(browserType, origin) {
    const browser = await browserType.launch({ timeout: 30_000 })
    try {
      const context = await browser.newContext({ locale: 'fr-FR', colorScheme: 'dark', reducedMotion: 'reduce' })
      const page = await context.newPage()
      for (const language of WebApiGate.everyEngineLanguages) {
        const label = `${browserType.name()} ${language}`
        await WebApiGate.open(page, origin, language)
        await WebApiGate.check(page, language, label)
        WebApiGate.expect(await WebApiGate.read(page, language, 'motion'), 'reduce', `${label}: Preferences gives the motion setting`)
        WebApiGate.expect(await WebApiGate.settled(page, language, 'scheme', 'dark'), 'dark', `${label}: a replay listener hears the current color scheme`)
        await page.emulateMedia({ colorScheme: 'light' })
        WebApiGate.expect(await WebApiGate.settled(page, language, 'scheme', 'light'), 'light', `${label}: PreferencesChanged reaches the listener`)
        await page.emulateMedia({ colorScheme: 'dark' })
        await context.setOffline(true)
        WebApiGate.expect(await WebApiGate.settled(page, language, 'online', false), false, `${label}: ConnectivityChanged reaches the listener`)
        await context.setOffline(false)
        await WebApiGate.checkUnitMethod(page, language, label)
      }
    } finally {
      await browser.close()
    }
  }

  static async main() {
    const project = mkdtempSync(path.join(tmpdir(), 'tachyon-web-apis-'))
    const server = createServer(WebApiGate.handler(project))
    let browser
    try {
      WebApiGate.write(project)
      WebApiGate.build(project)
      await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve))
      browser = await chromium.launch({ timeout: 30_000 })
      const origin = `http://127.0.0.1:${server.address().port}`
      const context = await browser.newContext({ locale: 'fr-FR', colorScheme: 'dark', reducedMotion: 'reduce', geolocation: WebApiGate.position })
      await context.grantPermissions(WebApiGate.permissions, { origin })
      const page = await context.newPage()
      page.on('console', (message) => WebApiGate.consoleLines.push(message.text()))
      for (const language of WebApiGate.languages) {
        await WebApiGate.open(page, origin, language)
        const refusals = await WebApiGate.refusals(page, language)
        WebApiGate.expect(refusals.active, false, `${language}: the page has no user action after the activation lifespan`)
        WebApiGate.expect(refusals.answer, 'The clipboard browser API needs a user action. Call it from a click or key handler.', `${language}: Clipboard refuses a call without a user action`)
        await WebApiGate.check(page, language)
        await WebApiGate.checkPreferences(page, context, language)
        await WebApiGate.checkPrompts(page, language)
        if (WebApiGate.everyEngineLanguages.includes(language)) await WebApiGate.checkOwnLanguage(page, language)
      }
      for (const browserType of WebApiGate.otherEngines) {
        if (WebApiGate.everyEngineLanguages.length > 0) await WebApiGate.checkEngine(browserType, origin)
      }
    } finally {
      await browser?.close()
      server.close()
      rmSync(project, { recursive: true, force: true })
    }
  }
}

await WebApiGate.main()
