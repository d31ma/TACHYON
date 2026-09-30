#![allow(clippy::expect_used)]

use super::WebLanguage;
use std::path::Path;
use std::process::Command;

struct WebApis;

impl WebApis {
    const RUNTIME: &str = include_str!("../template/tac-web.js");
    const MOST_RUNTIME_BYTES: usize = 9_500;

    const RUNTIME_DRIVER: &str = ";(0, eval)(await readFile('./tac-web.js', 'utf8'))\n\
         const web = globalThis[Symbol.for('tachyon.tac.web')]\n\
         const ask = (api, call, args) => JSON.parse(web.host(JSON.stringify({ api, call, args })))\n\
         const report = {\n\
         \x20 before: Date.now(),\n\
         \x20 now: ask('clock', 'now').value,\n\
         \x20 monotonic: ask('clock', 'monotonic').value,\n\
         \x20 zone: ask('clock', 'timeZone').value,\n\
         \x20 bytes: ask('secureRandom', 'bytes', [16]).value,\n\
         \x20 uuid: ask('secureRandom', 'uuid').value,\n\
         \x20 tooMany: ask('secureRandom', 'bytes', [65537]).error,\n\
         \x20 unknown: ask('clipboard', 'constructor').error,\n\
         \x20 inherited: ask('clock', 'toString').error,\n\
         \x20 broken: JSON.parse(web.host('{')).error,\n\
         \x20 resources: [typeof web.clock.now, typeof web.secureRandom.uuid],\n\
         }\n\
         report.after = Date.now()\n\
         console.log(JSON.stringify(report))\n";

    const RUST_SERVICE: &str = "#[Service]\n#[derive(Default)]\nstruct StampService {\n    \
         clock: Clock,\n    secure_random: SecureRandom,\n}\n\n\
         impl StampService {\n    \
         fn now(&self) -> f64 {\n        self.clock.now_ms()\n    }\n\n    \
         fn zone(&self) -> String {\n        self.clock.time_zone().unwrap_or_default()\n    }\n\n    \
         fn token(&self) -> String {\n        self.secure_random.uuid().unwrap_or_default()\n    }\n\n    \
         fn count(&self) -> i64 {\n        self.secure_random.bytes(16).map_or(-1, |bytes| bytes.len() as i64)\n    }\n\n    \
         fn refused(&self) -> String {\n        self.secure_random.bytes(65537).err().unwrap_or_default()\n    }\n}\n";

    const GO_SERVICE: &str = "// @Service\ntype StampService struct {\n\t// @Needs\n\tclock Clock\n\t// @Needs\n\tsecureRandom SecureRandom\n}\n\n\
         func (stamp *StampService) Now() float64 {\n\tvalue, _ := stamp.clock.NowMs()\n\treturn value\n}\n\n\
         func (stamp *StampService) Zone() string {\n\tvalue, _ := stamp.clock.TimeZone()\n\treturn value\n}\n\n\
         func (stamp *StampService) Token() string {\n\tvalue, _ := stamp.secureRandom.UUID()\n\treturn value\n}\n\n\
         func (stamp *StampService) Count() int {\n\tbytes, _ := stamp.secureRandom.Bytes(16)\n\treturn len(bytes)\n}\n\n\
         func (stamp *StampService) Refused() string {\n\t_, err := stamp.secureRandom.Bytes(65537)\n\treturn err.Error()\n}\n";

    const SWIFT_SERVICE: &str = "@Service\nfinal class StampService {\n    private let clock = Clock()\n    private let secureRandom = SecureRandom()\n\n    \
         func now() -> Double {\n        clock.nowMs()\n    }\n\n    \
         func zone() throws -> String {\n        try clock.timeZone()\n    }\n\n    \
         func token() throws -> String {\n        try secureRandom.uuid()\n    }\n\n    \
         func count() -> Int {\n        (try? secureRandom.bytes(16).count) ?? -1\n    }\n\n    \
         func refused() -> String {\n        do {\n            _ = try secureRandom.bytes(65537)\n            return \"\"\n        } catch {\n            return String(describing: error)\n        }\n    }\n}\n";

    const CSHARP_SERVICE: &str = "[Service]\npublic class StampService\n{\n    \
         private readonly Clock clock = new();\n    private readonly SecureRandom secureRandom = new();\n\n    \
         public double Now() => clock.NowMs();\n\n    \
         public string Zone() => clock.TimeZone();\n\n    \
         public string Token() => secureRandom.Uuid();\n\n    \
         public int Count() => secureRandom.Bytes(16).Length;\n\n    \
         public string Refused()\n    {\n        try\n        {\n            secureRandom.Bytes(65537);\n            return \"\";\n        }\n        \
         catch (System.InvalidOperationException error)\n        {\n            return error.Message;\n        }\n    }\n}\n";

    const DART_SERVICE: &str = "@Service()\nclass StampService {\n  @Needs()\n  final Clock clock;\n  @Needs()\n  final SecureRandom secureRandom;\n\n  \
         double now() => clock.nowMs();\n\n  \
         String zone() => clock.timeZone();\n\n  \
         String token() => secureRandom.uuid();\n\n  \
         int count() => secureRandom.bytes(16).length;\n\n  \
         String refused() {\n    try {\n      secureRandom.bytes(65537);\n      return '';\n    } on StateError catch (error) {\n      return error.message;\n    }\n  }\n}\n";

    const RUBY_SERVICE: &str = "#[Service]\nclass StampService\n  \
         def now\n    Clock.new.now_ms\n  end\n\n  \
         def zone\n    Clock.new.time_zone\n  end\n\n  \
         def token\n    SecureRandom.new.uuid\n  end\n\n  \
         def count\n    SecureRandom.new.bytes(16).length\n  end\n\n  \
         def refused\n    SecureRandom.new.bytes(65537)\n    \"\"\n  rescue RuntimeError => error\n    error.message\n  end\nend\n";

    const RUST_LISTENER: &str = r"#[Service]
#[derive(Default)]
struct StampService {
    connectivity: Connectivity,
    page_visibility: PageVisibility,
    preferences: Preferences,
    online: bool,
    visible: bool,
    scheme: String,
    language: String,
}

impl StampService {
    fn reachable(&self) -> bool {
        self.connectivity.online().unwrap_or(true)
    }

    fn shown(&self) -> bool {
        self.page_visibility.visible().unwrap_or(true)
    }

    fn theme(&self) -> String {
        self.preferences.scheme().unwrap_or_default()
    }

    fn motion(&self) -> String {
        self.preferences.motion().unwrap_or_default()
    }

    fn locale(&self) -> String {
        self.preferences.language().unwrap_or_default()
    }

    #[OnEvent]
    fn connectivity_changed(&mut self, changed: ConnectivityChanged) {
        self.online = changed.online;
    }

    #[OnEvent]
    fn visibility_changed(&mut self, changed: VisibilityChanged) {
        self.visible = changed.visible;
    }

    #[OnEvent]
    fn preferences_changed(&mut self, changed: PreferencesChanged) {
        self.scheme = changed.scheme;
        self.language = changed.language;
    }
}
";

    const SWIFT_LISTENER: &str = r#"@Service
final class StampService {
    private let connectivity = Connectivity()
    private let pageVisibility = PageVisibility()
    private let preferences = Preferences()
    var online: Bool = false
    var visible: Bool = false
    var scheme: String = ""
    var language: String = ""

    func reachable() throws -> Bool {
        try connectivity.online()
    }

    func shown() throws -> Bool {
        try pageVisibility.visible()
    }

    func theme() throws -> String {
        try preferences.scheme()
    }

    func motion() throws -> String {
        try preferences.motion()
    }

    func locale() throws -> String {
        try preferences.language()
    }

    @OnEvent
    func connectivityChanged(_ changed: ConnectivityChanged) {
        online = changed.online
    }

    @OnEvent
    func visibilityChanged(_ changed: VisibilityChanged) {
        visible = changed.visible
    }

    @OnEvent
    func preferencesChanged(_ changed: PreferencesChanged) {
        scheme = changed.scheme
        language = changed.language
    }
}
"#;

    const GO_LISTENER: &str = "// @Service
type StampService struct {
	// @Needs
	connectivity Connectivity
	// @Needs
	pageVisibility PageVisibility
	// @Needs
	preferences Preferences
	Online   bool
	Visible  bool
	Scheme   string
	Language string
}

func (stamp *StampService) Reachable() bool {
	value, _ := stamp.connectivity.Online()
	return value
}

func (stamp *StampService) Shown() bool {
	value, _ := stamp.pageVisibility.Visible()
	return value
}

func (stamp *StampService) Theme() string {
	value, _ := stamp.preferences.Scheme()
	return value
}

func (stamp *StampService) Motion() string {
	value, _ := stamp.preferences.Motion()
	return value
}

func (stamp *StampService) Locale() string {
	value, _ := stamp.preferences.Language()
	return value
}

// @OnEvent
func (stamp *StampService) HearConnectivity(changed ConnectivityChanged) {
	stamp.Online = changed.Online
}

// @OnEvent
func (stamp *StampService) HearVisibility(changed VisibilityChanged) {
	stamp.Visible = changed.Visible
}

// @OnEvent
func (stamp *StampService) HearPreferences(changed PreferencesChanged) {
	stamp.Scheme = changed.Scheme
	stamp.Language = changed.Language
}
";

    const CSHARP_LISTENER: &str = r#"[Service]
public class StampService
{
    private readonly Connectivity connectivity = new();
    private readonly PageVisibility pageVisibility = new();
    private readonly Preferences preferences = new();
    public bool Online = false;
    public bool Visible = false;
    public string Scheme = "";
    public string Language = "";

    public bool Reachable() => connectivity.Online();

    public bool Shown() => pageVisibility.Visible();

    public string Theme() => preferences.Scheme();

    public string Motion() => preferences.Motion();

    public string Locale() => preferences.Language();

    [OnEvent]
    public void HearConnectivity(ConnectivityChanged changed)
    {
        Online = changed.Online;
    }

    [OnEvent]
    public void HearVisibility(VisibilityChanged changed)
    {
        Visible = changed.Visible;
    }

    [OnEvent]
    public void HearPreferences(PreferencesChanged changed)
    {
        Scheme = changed.Scheme;
        Language = changed.Language;
    }
}
"#;

    const DART_LISTENER: &str = r"@Service()
class StampService {
  @Needs()
  final Connectivity connectivity;
  @Needs()
  final PageVisibility pageVisibility;
  @Needs()
  final Preferences preferences;
  bool online = false;
  bool visible = false;
  String scheme = '';
  String language = '';

  bool reachable() => connectivity.online();

  bool shown() => pageVisibility.visible();

  String theme() => preferences.scheme();

  String motion() => preferences.motion();

  String locale() => preferences.language();

  @OnEvent
  void connectivityChanged(ConnectivityChanged changed) {
    online = changed.online;
  }

  @OnEvent
  void visibilityChanged(VisibilityChanged changed) {
    visible = changed.visible;
  }

  @OnEvent
  void preferencesChanged(PreferencesChanged changed) {
    scheme = changed.scheme;
    language = changed.language;
  }
}
";

    const RUBY_LISTENER: &str = r#"#[Service]
class StampService
  attr_accessor :online, :visible, :scheme, :language

  def initialize
    @online = false
    @visible = false
    @scheme = ""
    @language = ""
  end

  def reachable
    Connectivity.new.online
  end

  def shown
    PageVisibility.new.visible
  end

  def theme
    Preferences.new.scheme
  end

  def motion
    Preferences.new.motion
  end

  def locale
    Preferences.new.language
  end

  # @param changed [ConnectivityChanged]
  #[OnEvent]
  def connectivity_changed(changed)
    @online = changed.online
  end

  # @param changed [VisibilityChanged]
  #[OnEvent]
  def visibility_changed(changed)
    @visible = changed.visible
  end

  # @param changed [PreferencesChanged]
  #[OnEvent]
  def preferences_changed(changed)
    @scheme = changed.scheme
    @language = changed.language
  end
end
"#;

    const BROWSER_STUBS: &str = "Object.defineProperty(globalThis, 'navigator', { configurable: true, value: { onLine: false, language: 'fr' } })
globalThis.document = { visibilityState: 'hidden', addEventListener: (type, listener) => { watched[`document:${type}`] = listener } }
globalThis.addEventListener = (type, listener) => { watched[`window:${type}`] = listener }
const media = { '(prefers-color-scheme: dark)': { matches: true }, '(prefers-reduced-motion: reduce)': { matches: false } }
for (const [query, list] of Object.entries(media)) list.addEventListener = (type, listener) => { watched[`${query}:${type}`] = listener }
globalThis.matchMedia = (query) => media[query]
";

    const WATCH_DRIVER: &str = "const watched = {}
__BROWSER_STUBS__
;(0, eval)(`${await readFile('./tac-web.js', 'utf8')}\nglobalThis.TacWebApis = TacWebApis`)
const web = globalThis[Symbol.for('tachyon.tac.web')]
const ask = (api, call) => JSON.parse(web.host(JSON.stringify({ api, call, args: [] }))).value
const values = { online: ask('connectivity', 'online'), visible: ask('pageVisibility', 'visible'), scheme: ask('preferences', 'scheme'), motion: ask('preferences', 'motion'), language: ask('preferences', 'language') }
const published = []
TacWebApis.watch({ publish: (name, value) => published.push([name, value]), send: () => {} })
navigator.onLine = true
watched['window:online']()
document.visibilityState = 'visible'
watched['document:visibilitychange']()
media['(prefers-color-scheme: dark)'].matches = false
watched['(prefers-color-scheme: dark):change']()
console.log(JSON.stringify({ values, published, watched: Object.keys(watched).sort() }))
";

    const GRANT_STUBS: &str = r#"const effects = { written: [], shown: [], sent: [] }
const grants = { content: 'clipboard share notifications geolocation' }
globalThis.document = { visibilityState: 'visible', addEventListener: () => {}, querySelector: (selector) => selector === 'meta[name="tachyon-browser-apis"]' ? grants : null }
const activation = { isActive: true }
Object.defineProperty(globalThis, 'navigator', { configurable: true, value: {
  onLine: true, language: 'en', userActivation: activation,
  clipboard: { writeText: async (text) => { if (text === 'refused') throw new Error('Write denied'); effects.written.push(text) }, readText: async () => 'from the clipboard' },
  share: async (data) => { effects.shared = data },
  geolocation: { getCurrentPosition: (found, failed, options) => { effects.timeout = options.timeout; found({ coords: { latitude: 48.85, longitude: 2.35, accuracy: 12 }, timestamp: 1700000000000 }) } },
} })
globalThis.Notification = class { static permission = 'default'; static requestPermission = async () => 'granted'; constructor(title, options) { effects.shown.push([title, options.body]) } }
"#;

    const GRANTS_DRIVER: &str = r"__GRANT_STUBS__
;(0, eval)(`${await readFile('./tac-web.js', 'utf8')}\nglobalThis.TacWebApis = TacWebApis`)
const web = globalThis[Symbol.for('tachyon.tac.web')]
TacWebApis.events = { publish: (name, value) => effects.sent.push(['everyone', name, value]), send: (owner, name, value) => effects.sent.push([owner?.name ?? 'everyone', name, value]) }
const ask = (api, call, args = []) => JSON.parse(web.host(JSON.stringify({ api, call, args })))
const owner = { name: 'caller' }
const asOwner = (api, call, args) => web.calling(owner, () => ask(api, call, args))
const answers = {}
answers.write = asOwner('clipboard', 'writeText', ['copied'])
answers.refusedWrite = asOwner('clipboard', 'writeText', ['refused'])
answers.read = asOwner('clipboard', 'readText')
answers.share = asOwner('share', 'open', ['Title', 'Text', 'https://example.com/'])
answers.permission = ask('notifications', 'permission')
answers.early = ask('notifications', 'show', ['Hello', 'Body'])
answers.request = asOwner('notifications', 'requestPermission')
answers.position = ask('geolocation', 'currentPosition')
Notification.permission = 'granted'
answers.show = ask('notifications', 'show', ['Hello', 'Body'])
activation.isActive = false
answers.noGesture = asOwner('clipboard', 'readText')
answers.locateWithoutGesture = ask('geolocation', 'currentPosition')
grants.content = 'share'
answers.notGranted = asOwner('clipboard', 'readText')
answers.permissionNotGranted = ask('notifications', 'permission')
await new Promise((resolve) => setTimeout(resolve, 0))
console.log(JSON.stringify({ answers, effects }))
";

    const WINDOW_DRIVER: &str = r"globalThis.document = { visibilityState: 'visible', addEventListener: () => {}, querySelector: () => null }
;(0, eval)(await readFile('./tac-web.js', 'utf8'))
const web = globalThis[Symbol.for('tachyon.tac.web')]
const ask = (call, args = []) => JSON.parse(web.host(JSON.stringify({ api: 'windowControls', call, args })))
const answers = { tabGranted: ask('granted', ['minimize']), tabMinimize: ask('minimize') }
const sent = []
globalThis.__tachyonNativeHostCall = (capability, payload) => { sent.push([capability, JSON.parse(payload)]) }
globalThis.__tachyonWindowControls = Object.freeze(['minimize', 'resize', 'title'])
answers.granted = ask('granted', ['minimize'])
answers.closeGranted = ask('granted', ['close'])
answers.minimize = ask('minimize')
answers.close = ask('close')
answers.resize = ask('resize', [1280, 800])
answers.tooSmall = ask('resize', [10, 800])
answers.title = ask('setTitle', ['Report'])
console.log(JSON.stringify({ answers, sent }))
";

    const RUST_PROMPTS: &str = r#"#[Service]
#[derive(Default)]
struct StampService {
    clipboard: Clipboard,
    share: Share,
    notifications: Notifications,
    geolocation: Geolocation,
    pasted: String,
    shared: bool,
    permission: String,
    latitude: f64,
    failure: String,
}

impl StampService {
    fn copy(&self, text: &str) {
        let _ = self.clipboard.write_text(text);
    }

    fn paste(&self) {
        let _ = self.clipboard.read_text();
    }

    fn tell(&self) {
        let _ = self.share.open("Title", "Text", "https://example.com/");
    }

    fn ask(&self) {
        let _ = self.notifications.request_permission();
    }

    fn notify(&self) {
        let _ = self.notifications.show("Hello", "Body");
    }

    fn locate(&self) {
        let _ = self.geolocation.current_position();
    }

    fn denied(&self) -> String {
        self.notifications.show("Hello", "Body").err().unwrap_or_default()
    }

    #[OnEvent]
    fn hear_clipboard(&mut self, read: ClipboardTextRead) {
        self.pasted = read.text;
    }

    #[OnEvent]
    fn hear_share(&mut self, finished: ShareFinished) {
        self.shared = finished == ShareFinished {};
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
    fn hear_failure(&mut self, failed: BrowserCallFailed) {
        self.failure = format!("{}.{}: {}", failed.api, failed.call, failed.message);
    }
}
"#;

    const GO_PROMPTS: &str = r#"// @Service
type StampService struct {
	// @Needs
	clipboard Clipboard
	// @Needs
	share Share
	// @Needs
	notifications Notifications
	// @Needs
	geolocation Geolocation
	Pasted     string
	Shared     bool
	Permission string
	Latitude   float64
	Failure    string
}

func (stamp *StampService) Copy(text string) {
	_ = stamp.clipboard.WriteText(text)
}

func (stamp *StampService) Paste() {
	_ = stamp.clipboard.ReadText()
}

func (stamp *StampService) Tell() {
	_ = stamp.share.Open("Title", "Text", "https://example.com/")
}

func (stamp *StampService) Ask() {
	_ = stamp.notifications.RequestPermission()
}

func (stamp *StampService) Notify() {
	_ = stamp.notifications.Show("Hello", "Body")
}

func (stamp *StampService) Locate() {
	_ = stamp.geolocation.CurrentPosition()
}

func (stamp *StampService) Denied() string {
	err := stamp.notifications.Show("Hello", "Body")
	if err == nil {
		return ""
	}
	return err.Error()
}

// @OnEvent
func (stamp *StampService) HearClipboard(read ClipboardTextRead) {
	stamp.Pasted = read.Text
}

// @OnEvent
func (stamp *StampService) HearShare(finished ShareFinished) {
	stamp.Shared = finished == ShareFinished{}
}

// @OnEvent
func (stamp *StampService) HearPermission(changed NotificationPermissionChanged) {
	stamp.Permission = changed.Permission
}

// @OnEvent
func (stamp *StampService) HearPosition(found PositionFound) {
	stamp.Latitude = found.Latitude
}

// @OnEvent
func (stamp *StampService) HearFailure(failed BrowserCallFailed) {
	stamp.Failure = failed.Api + "." + failed.Call + ": " + failed.Message
}
"#;

    const SWIFT_PROMPTS: &str = r#"@Service
final class StampService {
    private let clipboard = Clipboard()
    private let share = Share()
    private let notifications = Notifications()
    private let geolocation = Geolocation()
    var pasted: String = ""
    var shared: Bool = false
    var permission: String = ""
    var latitude: Double = 0
    var failure: String = ""

    func copy(_ text: String) {
        try? clipboard.writeText(text)
    }

    func paste() throws {
        try clipboard.readText()
    }

    func tell() throws {
        try share.open(title: "Title", text: "Text", url: "https://example.com/")
    }

    func ask() throws {
        try notifications.requestPermission()
    }

    func notify() throws {
        try notifications.show(title: "Hello", body: "Body")
    }

    func locate() throws {
        try geolocation.currentPosition()
    }

    func denied() -> String {
        do {
            try notifications.show(title: "Hello", body: "Body")
            return ""
        } catch {
            return String(describing: error)
        }
    }

    @OnEvent
    func hearClipboard(_ read: ClipboardTextRead) {
        pasted = read.text
    }

    @OnEvent
    func hearShare(_ finished: ShareFinished) {
        shared = true
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
    func hearFailure(_ failed: BrowserCallFailed) {
        failure = failed.api + "." + failed.call + ": " + failed.message
    }
}
"#;

    const JAVA_SERVICE: &str = r#"@Service
class StampService {
    private final Clock clock = new Clock();
    private final SecureRandom secureRandom = new SecureRandom();

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

    String refused() {
        try {
            secureRandom.bytes(65537);
            return "";
        } catch (IllegalStateException refusal) {
            return refusal.getMessage();
        }
    }
}
"#;

    const JAVA_LISTENER: &str = r#"@Service
class StampService {
    private final Connectivity connectivity = new Connectivity();
    private final PageVisibility pageVisibility = new PageVisibility();
    private final Preferences preferences = new Preferences();
    boolean online = false;
    boolean visible = false;
    String scheme = "";
    String language = "";

    boolean reachable() {
        return connectivity.online();
    }

    boolean shown() {
        return pageVisibility.visible();
    }

    String theme() {
        return preferences.scheme();
    }

    String motion() {
        return preferences.motion();
    }

    String locale() {
        return preferences.language();
    }

    @OnEvent
    void connectivityChanged(ConnectivityChanged changed) {
        online = changed.online();
    }

    @OnEvent
    void visibilityChanged(VisibilityChanged changed) {
        visible = changed.visible();
    }

    @OnEvent
    void preferencesChanged(PreferencesChanged changed) {
        scheme = changed.scheme();
        language = changed.language();
    }
}
"#;

    const JAVA_PROMPTS: &str = r#"@Service
class StampService {
    private final Clipboard clipboard = new Clipboard();
    private final Share share = new Share();
    private final Notifications notifications = new Notifications();
    private final Geolocation geolocation = new Geolocation();
    String pasted = "";
    boolean shared = false;
    String permission = "";
    double latitude = 0;
    String failure = "";

    void copy(String text) {
        clipboard.writeText(text);
    }

    void paste() {
        clipboard.readText();
    }

    void tell() {
        share.open("Title", "Text", "https://example.com/");
    }

    void ask() {
        notifications.requestPermission();
    }

    void notifyPage() {
        notifications.show("Hello", "Body");
    }

    void locate() {
        geolocation.currentPosition();
    }

    String denied() {
        try {
            notifications.show("Hello", "Body");
            return "";
        } catch (IllegalStateException refusal) {
            return refusal.getMessage();
        }
    }

    @OnEvent
    void hearClipboard(ClipboardTextRead read) {
        pasted = read.text();
    }

    @OnEvent
    void hearShare(ShareFinished finished) {
        shared = true;
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
    void hearFailure(BrowserCallFailed failed) {
        failure = failed.api() + "." + failed.call() + ": " + failed.message();
    }
}
"#;

    const CSHARP_PROMPTS: &str = r#"[Service]
public class StampService
{
    private readonly Clipboard clipboard = new();
    private readonly Share share = new();
    private readonly Notifications notifications = new();
    private readonly Geolocation geolocation = new();
    public string Pasted = "";
    public bool Shared = false;
    public string Permission = "";
    public double Latitude = 0;
    public string Failure = "";

    public void Copy(string text) => clipboard.WriteText(text);

    public void Paste() => clipboard.ReadText();

    public void Tell() => share.Open("Title", "Text", "https://example.com/");

    public void Ask() => notifications.RequestPermission();

    public void Notify() => notifications.Show("Hello", "Body");

    public void Locate() => geolocation.CurrentPosition();

    public string Denied()
    {
        try
        {
            notifications.Show("Hello", "Body");
            return "";
        }
        catch (System.InvalidOperationException error)
        {
            return error.Message;
        }
    }

    [OnEvent]
    public void HearClipboard(ClipboardTextRead read)
    {
        Pasted = read.Text;
    }

    [OnEvent]
    public void HearShare(ShareFinished finished)
    {
        Shared = finished == new ShareFinished();
    }

    [OnEvent]
    public void HearPermission(NotificationPermissionChanged changed)
    {
        Permission = changed.Permission;
    }

    [OnEvent]
    public void HearPosition(PositionFound found)
    {
        Latitude = found.Latitude;
    }

    [OnEvent]
    public void HearFailure(BrowserCallFailed failed)
    {
        Failure = $"{failed.Api}.{failed.Call}: {failed.Message}";
    }
}
"#;

    const DART_PROMPTS: &str = r"@Service()
class StampService {
  @Needs()
  final Clipboard clipboard;
  @Needs()
  final Share share;
  @Needs()
  final Notifications notifications;
  @Needs()
  final Geolocation geolocation;
  String pasted = '';
  bool shared = false;
  String permission = '';
  double latitude = 0;
  String failure = '';

  void copy(String text) => clipboard.writeText(text);

  void paste() => clipboard.readText();

  void tell() => share.open('Title', 'Text', 'https://example.com/');

  void ask() => notifications.requestPermission();

  void notify() => notifications.show('Hello', 'Body');

  void locate() => geolocation.currentPosition();

  String denied() {
    try {
      notifications.show('Hello', 'Body');
      return '';
    } on StateError catch (error) {
      return error.message;
    }
  }

  @OnEvent
  void hearClipboard(ClipboardTextRead read) {
    pasted = read.text;
  }

  @OnEvent
  void hearShare(ShareFinished finished) {
    shared = true;
  }

  @OnEvent
  void hearPermission(NotificationPermissionChanged changed) {
    permission = changed.permission;
  }

  @OnEvent
  void hearPosition(PositionFound found) {
    latitude = found.latitude;
  }

  @OnEvent
  void hearFailure(BrowserCallFailed failed) {
    failure = '${failed.api}.${failed.call}: ${failed.message}';
  }
}
";

    const RUBY_PROMPTS: &str = r##"#[Service]
class StampService
  attr_accessor :pasted, :shared, :permission, :latitude, :failure

  def initialize
    @pasted = ""
    @shared = false
    @permission = ""
    @latitude = 0.0
    @failure = ""
  end

  def copy(text)
    Clipboard.new.write_text(text)
  end

  def paste
    Clipboard.new.read_text
  end

  def tell
    Share.new.open("Title", "Text", "https://example.com/")
  end

  def ask
    Notifications.new.request_permission
  end

  def notify
    Notifications.new.show("Hello", "Body")
  end

  def locate
    Geolocation.new.current_position
  end

  def denied
    Notifications.new.show("Hello", "Body")
    ""
  rescue RuntimeError => error
    error.message
  end

  # @param read [ClipboardTextRead]
  #[OnEvent]
  def hear_clipboard(read)
    @pasted = read.text
  end

  # @param finished [ShareFinished]
  #[OnEvent]
  def hear_share(finished)
    @shared = finished.is_a?(ShareFinished)
  end

  # @param changed [NotificationPermissionChanged]
  #[OnEvent]
  def hear_permission(changed)
    @permission = changed.permission
  end

  # @param found [PositionFound]
  #[OnEvent]
  def hear_position(found)
    @latitude = found.latitude
  end

  # @param failed [BrowserCallFailed]
  #[OnEvent]
  def hear_failure(failed)
    @failure = "#{failed.api}.#{failed.call}: #{failed.message}"
  end
end
"##;

    const PROMPTS_DRIVER: &str = r";(0, eval)(`${await readFile('./tac-web.js', 'utf8')}\nglobalThis.TacWebApis = TacWebApis`)
const { default: TacApplicationEvents } = await import('./application-events.mjs')
const { default: StampService, __tachyon_event_plan__: plan } = await import('./StampService.js')
__GRANT_STUBS__
const bus = new TacApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })
TacWebApis.events = { publish: (name, value) => bus.publish(name, value), send: (owner, name, value) => bus.send(owner, name, value) }
const other = { heard: [], hearClipboard(value) { this.heard.push(value) } }
bus.attach(other, { listeners: [{ method: 'hearClipboard', event: 'ClipboardTextRead' }] }, {})
const stamp = new StampService({})
bus.attach(stamp, plan.types[0], { applicationEventPublisher: null })
await bus.initialize(stamp)
const denied = stamp.denied()
stamp.copy('refused')
stamp.paste()
stamp.tell()
stamp.ask()
stamp.locate()
Notification.permission = 'granted'
stamp.notify()
await new Promise((resolve) => setTimeout(resolve, 0))
console.log(JSON.stringify({ denied, pasted: stamp.pasted, shared: stamp.shared, permission: stamp.permission, latitude: stamp.latitude, failure: stamp.failure, shown: effects.shown, other: other.heard }))
process.exit(0)
";

    const EVENTS_DRIVER: &str = ";(0, eval)(await readFile('./tac-web.js', 'utf8'))
const watched = {}
const { default: TacApplicationEvents } = await import('./application-events.mjs')
const { default: StampService, __tachyon_event_plan__: plan } = await import('./StampService.js')
const bus = new TacApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })
const stamp = new StampService({})
bus.attach(stamp, plan.types[0], { applicationEventPublisher: null })
await bus.initialize(stamp)
__BROWSER_STUBS__
bus.publish('ConnectivityChanged', { online: true })
bus.publish('VisibilityChanged', { visible: true })
bus.publish('PreferencesChanged', { scheme: 'light', motion: 'reduce', language: 'de' })
const heard = { online: stamp.online, visible: stamp.visible, scheme: stamp.scheme, language: stamp.language }
const read = { reachable: stamp.reachable(), shown: stamp.shown(), theme: stamp.theme(), motion: stamp.motion(), locale: stamp.locale() }
console.log(JSON.stringify({ heard, read }))
process.exit(0)
";

    const EVENTS_RUNTIME: &str = concat!(
        include_str!("../template/application-events.js"),
        "\nexport default TacApplicationEvents\n"
    );

    const FILE_FETCH: &str =
        "globalThis.fetch = async (url) => new Response(await readFile(url))\n";

    const SERVICE_DRIVER: &str = ";(0, eval)(await readFile('./tac-web.js', 'utf8'))\n\
         const { default: StampService } = await import('./StampService.js')\n\
         const stamp = new StampService({})\n\
         const before = Date.now()\n\
         const report = { before, now: stamp.now(), zone: stamp.zone(), token: stamp.token(), count: stamp.count(), refused: stamp.refused(), keys: Object.keys(stamp) }\n\
         report.after = Date.now()\n\
         console.log(JSON.stringify(report))\n\
         process.exit(0)\n";

    fn node_available() -> bool {
        Command::new("node")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
    }

    fn rust_web_target() -> bool {
        let Ok(output) = Command::new("rustc")
            .args([
                "--print",
                "target-libdir",
                "--target",
                crate::native::RustCompanion::WEB_TARGET,
            ])
            .output()
        else {
            return false;
        };
        Path::new(String::from_utf8_lossy(&output.stdout).trim()).is_dir()
    }

    fn write(root: &Path, relative: &str, bytes: &[u8]) -> std::path::PathBuf {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(&path, bytes).expect("write");
        path
    }

    fn run(root: &Path, driver: &str) -> serde_json::Value {
        let script = format!("import {{ readFile }} from 'node:fs/promises'\n{driver}");
        Self::write(root, "drive.mjs", script.as_bytes());
        Self::write(root, "tac-web.js", Self::RUNTIME.as_bytes());
        let output = Command::new("node")
            .arg("drive.mjs")
            .current_dir(root)
            .output()
            .expect("node");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "{stdout}{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_str(stdout.lines().last().unwrap_or_default()).expect("report")
    }

    fn assert_now(report: &serde_json::Value) {
        let now = report["now"].as_f64().expect("now");
        let before = report["before"].as_f64().expect("before");
        let after = report["after"].as_f64().expect("after");
        assert!(now >= before && now <= after, "{report}");
    }

    async fn publish(root: &Path, language: WebLanguage, extension: &str, source: &str) {
        let profile = crate::BuildProfile::Release;
        Self::publish_with(root, language, (extension, source), profile).await;
    }

    async fn publish_with(
        root: &Path,
        language: WebLanguage,
        (extension, source): (&str, &str),
        profile: crate::BuildProfile,
    ) {
        let relative = format!("client/services/StampService.{extension}");
        let path = Self::write(root, &relative, source.as_bytes());
        let services = [(
            String::from("stamp"),
            String::from("StampService"),
            language,
            path,
        )];
        let jobs = super::BuildJobs::new(1);
        let published = super::build_all(root, &services, &jobs, profile)
            .await
            .expect("build");
        for (name, bytes) in published.files {
            Self::write(root, &name, &bytes);
        }
    }

    fn assert_stamp(report: &serde_json::Value) {
        Self::assert_now(report);
        assert!(
            report["zone"].as_str().is_some_and(|zone| !zone.is_empty()),
            "{report}"
        );
        Self::assert_uuid(&report["token"]);
        assert_eq!(report["count"], 16, "{report}");
        assert_eq!(report["refused"], "SecureRandom gives 0 to 65536 bytes.");
        assert_eq!(report["keys"], serde_json::json!([]), "{report}");
    }

    fn dart_available() -> bool {
        Command::new("dart")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
    }

    async fn publish_dart(root: &Path, source: &str) {
        let path = Self::write(root, "client/services/StampService.dart", source.as_bytes());
        let services = [("StampService", path.as_path())];
        let instances = std::collections::BTreeSet::from([String::from("stamp")]);
        let modules = crate::tac_dart::build_all(root, &services, &instances, crate::BuildProfile::Release)
            .await
            .expect("build");
        for (name, bytes) in modules {
            Self::write(root, &name, &bytes);
        }
        Self::write(root, "package.json", b"{\"type\":\"module\"}");
    }

    fn assert_heard(root: &Path, fetches_files: bool) {
        Self::write(
            root,
            "application-events.mjs",
            Self::EVENTS_RUNTIME.as_bytes(),
        );
        let events = Self::EVENTS_DRIVER.replace("__BROWSER_STUBS__", Self::BROWSER_STUBS);
        let driver = if fetches_files {
            format!("{}{events}", Self::FILE_FETCH)
        } else {
            events
        };
        let report = Self::run(root, &driver);
        assert_eq!(
            report["heard"],
            serde_json::json!({"online": true, "visible": true, "scheme": "light", "language": "de"})
        );
        assert_eq!(
            report["read"],
            serde_json::json!({"reachable": false, "shown": false, "theme": "dark", "motion": "no-preference", "locale": "fr"})
        );
    }

    fn assert_answered(root: &Path, fetches_files: bool) {
        Self::assert_answered_by(root, fetches_files, Self::PROMPTS_DRIVER);
    }

    fn assert_answered_by(root: &Path, fetches_files: bool, driver: &str) {
        Self::write(
            root,
            "application-events.mjs",
            Self::EVENTS_RUNTIME.as_bytes(),
        );
        let prompts = driver.replace("__GRANT_STUBS__", Self::GRANT_STUBS);
        let driver = if fetches_files {
            format!("{}{prompts}", Self::FILE_FETCH)
        } else {
            prompts
        };
        let report = Self::run(root, &driver);
        assert_eq!(
            report,
            serde_json::json!({
                "denied": "Notifications are not allowed. Call requestPermission first.",
                "pasted": "from the clipboard",
                "shared": true,
                "permission": "granted",
                "latitude": 48.85,
                "failure": "clipboard.writeText: Write denied",
                "shown": [["Hello", "Body"]],
                "other": [],
            })
        );
    }

    fn assert_uuid(value: &serde_json::Value) {
        let text = value.as_str().expect("uuid");
        assert_eq!(text.len(), 36, "{text}");
        assert_eq!(&text[14..15], "4", "{text}");
        assert!("89ab".contains(&text[19..20]), "{text}");
    }
}

#[test]
fn the_page_runtime_stays_within_its_size_budget() {
    assert!(
        WebApis::RUNTIME.len() <= WebApis::MOST_RUNTIME_BYTES,
        "tac-web.js has {} bytes",
        WebApis::RUNTIME.len()
    );
}

#[test]
fn the_page_runtime_answers_clock_and_secure_random_calls() {
    if !WebApis::node_available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("root");
    let report = WebApis::run(root.path(), WebApis::RUNTIME_DRIVER);
    WebApis::assert_now(&report);
    assert!(report["monotonic"].as_f64().is_some(), "{report}");
    assert!(
        report["zone"].as_str().is_some_and(|zone| !zone.is_empty()),
        "{report}"
    );
    assert_eq!(
        report["bytes"].as_array().map(Vec::len),
        Some(16),
        "{report}"
    );
    WebApis::assert_uuid(&report["uuid"]);
    assert_eq!(report["tooMany"], "SecureRandom gives 0 to 65536 bytes.");
    assert_eq!(
        report["unknown"],
        "Tachyon has no browser call named 'clipboard.constructor'."
    );
    assert_eq!(
        report["inherited"],
        "Tachyon has no browser call named 'clock.toString'."
    );
    assert!(report["broken"].is_string(), "{report}");
    assert_eq!(
        report["resources"],
        serde_json::json!(["function", "function"])
    );
}

#[test]
fn the_page_runtime_answers_and_watches_connectivity_visibility_and_preferences() {
    if !WebApis::node_available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("root");
    let driver = WebApis::WATCH_DRIVER.replace("__BROWSER_STUBS__", WebApis::BROWSER_STUBS);
    let report = WebApis::run(root.path(), &driver);
    assert_eq!(
        report["values"],
        serde_json::json!({"online": false, "visible": false, "scheme": "dark", "motion": "no-preference", "language": "fr"})
    );
    assert_eq!(
        report["published"],
        serde_json::json!([
            ["ConnectivityChanged", {"online": false}],
            ["VisibilityChanged", {"visible": false}],
            ["PreferencesChanged", {"scheme": "dark", "motion": "no-preference", "language": "fr"}],
            ["ConnectivityChanged", {"online": true}],
            ["VisibilityChanged", {"visible": true}],
            ["PreferencesChanged", {"scheme": "light", "motion": "no-preference", "language": "fr"}],
        ])
    );
    assert_eq!(
        report["watched"],
        serde_json::json!([
            "(prefers-color-scheme: dark):change",
            "(prefers-reduced-motion: reduce):change",
            "document:visibilitychange",
            "window:languagechange",
            "window:offline",
            "window:online",
        ])
    );
}

#[test]
fn the_page_runtime_grants_checks_gestures_and_answers_its_caller_with_events() {
    if !WebApis::node_available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("root");
    let driver = WebApis::GRANTS_DRIVER.replace("__GRANT_STUBS__", WebApis::GRANT_STUBS);
    let report = WebApis::run(root.path(), &driver);
    let answers = &report["answers"];
    for started in [
        "write",
        "refusedWrite",
        "read",
        "share",
        "request",
        "position",
        "show",
        "locateWithoutGesture",
    ] {
        assert_eq!(
            answers[started],
            serde_json::json!({"value": null}),
            "{started}: {report}"
        );
    }
    assert_eq!(answers["permission"]["value"], "default");
    assert_eq!(
        answers["early"]["error"],
        "Notifications are not allowed. Call requestPermission first."
    );
    assert_eq!(
        answers["noGesture"]["error"],
        "The clipboard browser API needs a user action. Call it from a click or key handler."
    );
    assert_eq!(
        answers["notGranted"]["error"],
        "The clipboard browser API is not granted. Add \"clipboard\" to tac.browserApis in client/manifest.json."
    );
    assert_eq!(
        answers["permissionNotGranted"]["error"],
        "The notifications browser API is not granted. Add \"notifications\" to tac.browserApis in client/manifest.json."
    );
    let effects = &report["effects"];
    assert_eq!(effects["written"], serde_json::json!(["copied"]));
    assert_eq!(effects["shown"], serde_json::json!([["Hello", "Body"]]));
    assert_eq!(
        effects["shared"],
        serde_json::json!({"title": "Title", "text": "Text", "url": "https://example.com/"})
    );
    assert_eq!(effects["timeout"], 10000);
    let position = serde_json::json!({"latitude": 48.85, "longitude": 2.35, "accuracy": 12, "timestamp": 1_700_000_000_000_u64});
    assert_eq!(
        effects["sent"],
        serde_json::json!([
            ["caller", "BrowserCallFailed", {"api": "clipboard", "call": "writeText", "message": "Write denied"}],
            ["caller", "ClipboardTextRead", {"text": "from the clipboard"}],
            ["caller", "ShareFinished", {}],
            ["caller", "NotificationPermissionChanged", {"permission": "granted"}],
            ["everyone", "PositionFound", position],
            ["everyone", "PositionFound", position],
        ])
    );
}

#[test]
fn the_page_runtime_sends_only_granted_window_controls_to_the_native_host() {
    if !WebApis::node_available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("root");
    let report = WebApis::run(root.path(), WebApis::WINDOW_DRIVER);
    let answers = &report["answers"];
    assert_eq!(answers["tabGranted"], serde_json::json!({"value": false}));
    assert_eq!(
        answers["tabMinimize"]["error"],
        "The minimize window control needs the native app window."
    );
    assert_eq!(answers["granted"], serde_json::json!({"value": true}));
    assert_eq!(answers["closeGranted"], serde_json::json!({"value": false}));
    for sent in ["minimize", "resize", "title"] {
        assert_eq!(
            answers[sent],
            serde_json::json!({"value": null}),
            "{sent}: {report}"
        );
    }
    assert_eq!(
        answers["close"]["error"],
        "The close window control is not granted. Add \"close\" to tac.application.window.controls in client/manifest.json."
    );
    assert_eq!(
        answers["tooSmall"]["error"],
        "A window size is a whole number from 64 to 16384."
    );
    assert_eq!(
        report["sent"],
        serde_json::json!([
            ["window.minimize", {}],
            ["window.resize", {"width": 1280, "height": 800}],
            ["window.title", {"title": "Report"}],
        ])
    );
}

#[tokio::test]
async fn a_go_web_service_reads_the_clock_and_secure_random_from_the_page() {
    let go = Command::new("go")
        .arg("version")
        .output()
        .is_ok_and(|output| output.status.success());
    if !WebApis::node_available() || !go {
        eprintln!("skipping: node or go is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Go, "go", WebApis::GO_SERVICE).await;
    let driver = format!("{}{}", WebApis::FILE_FETCH, WebApis::SERVICE_DRIVER);
    let report = WebApis::run(root.path(), &driver);
    WebApis::assert_stamp(&report);
}

#[tokio::test]
async fn a_csharp_web_service_reads_the_clock_and_secure_random_from_the_page() {
    let workload = Command::new("dotnet")
        .args(["workload", "list"])
        .output()
        .is_ok_and(|output| String::from_utf8_lossy(&output.stdout).contains("wasm-tools"));
    if !WebApis::node_available() || !workload {
        eprintln!("skipping: node or the .NET wasm-tools workload is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = ("cs", WebApis::CSHARP_SERVICE);
    let profile = crate::BuildProfile::Development;
    WebApis::publish_with(root.path(), WebLanguage::CSharp, source, profile).await;
    let report = WebApis::run(root.path(), WebApis::SERVICE_DRIVER);
    WebApis::assert_stamp(&report);
}

#[tokio::test]
async fn a_dart_service_reads_the_clock_and_secure_random_from_the_page() {
    if !WebApis::node_available() || !WebApis::dart_available() {
        eprintln!("skipping: node or dart is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish_dart(root.path(), WebApis::DART_SERVICE).await;
    let driver = format!("{}{}", WebApis::FILE_FETCH, WebApis::SERVICE_DRIVER);
    let report = WebApis::run(root.path(), &driver);
    WebApis::assert_stamp(&report);
}

#[tokio::test]
async fn a_dart_service_reads_and_hears_connectivity_visibility_and_preferences() {
    if !WebApis::node_available() || !WebApis::dart_available() {
        eprintln!("skipping: node or dart is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish_dart(root.path(), WebApis::DART_LISTENER).await;
    WebApis::assert_heard(root.path(), true);
}

#[tokio::test]
async fn a_ruby_web_service_reads_the_clock_and_secure_random_from_the_page() {
    let unavailable = crate::tac_ruby::RubyRuntimeBuild::unavailable_reason();
    if !WebApis::node_available() || unavailable.is_some() {
        eprintln!("skipping: node or the Ruby runtime is unavailable: {unavailable:?}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Ruby, "rb", WebApis::RUBY_SERVICE).await;
    let driver = format!("{}{}", WebApis::FILE_FETCH, WebApis::SERVICE_DRIVER);
    let report = WebApis::run(root.path(), &driver);
    WebApis::assert_stamp(&report);
}

#[tokio::test]
async fn a_rust_web_service_reads_the_clock_and_secure_random_from_the_page() {
    if !WebApis::node_available() || !WebApis::rust_web_target() {
        eprintln!("skipping: node or the wasm32 Rust target is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Rust, "rs", WebApis::RUST_SERVICE).await;
    let report = WebApis::run(root.path(), WebApis::SERVICE_DRIVER);
    WebApis::assert_stamp(&report);
}

#[tokio::test]
async fn a_rust_web_service_reads_and_hears_connectivity_visibility_and_preferences() {
    if !WebApis::node_available() || !WebApis::rust_web_target() {
        eprintln!("skipping: node or the wasm32 Rust target is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Rust, "rs", WebApis::RUST_LISTENER).await;
    WebApis::assert_heard(root.path(), false);
}

#[tokio::test]
async fn a_go_web_service_reads_and_hears_connectivity_visibility_and_preferences() {
    let go = Command::new("go")
        .arg("version")
        .output()
        .is_ok_and(|output| output.status.success());
    if !WebApis::node_available() || !go {
        eprintln!("skipping: node or go is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Go, "go", WebApis::GO_LISTENER).await;
    WebApis::assert_heard(root.path(), true);
}

#[tokio::test]
async fn a_csharp_web_service_reads_and_hears_connectivity_visibility_and_preferences() {
    let workload = Command::new("dotnet")
        .args(["workload", "list"])
        .output()
        .is_ok_and(|output| String::from_utf8_lossy(&output.stdout).contains("wasm-tools"));
    if !WebApis::node_available() || !workload {
        eprintln!("skipping: node or the .NET wasm-tools workload is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = ("cs", WebApis::CSHARP_LISTENER);
    let profile = crate::BuildProfile::Development;
    WebApis::publish_with(root.path(), WebLanguage::CSharp, source, profile).await;
    WebApis::assert_heard(root.path(), false);
}

#[tokio::test]
async fn a_ruby_web_service_reads_and_hears_connectivity_visibility_and_preferences() {
    let unavailable = crate::tac_ruby::RubyRuntimeBuild::unavailable_reason();
    if !WebApis::node_available() || unavailable.is_some() {
        eprintln!("skipping: node or the Ruby runtime is unavailable: {unavailable:?}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Ruby, "rb", WebApis::RUBY_LISTENER).await;
    WebApis::assert_heard(root.path(), true);
}

#[tokio::test]
async fn a_rust_web_service_hears_the_answers_to_its_own_browser_calls() {
    if !WebApis::node_available() || !WebApis::rust_web_target() {
        eprintln!("skipping: node or the wasm32 Rust target is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Rust, "rs", WebApis::RUST_PROMPTS).await;
    WebApis::assert_answered(root.path(), false);
}

#[tokio::test]
async fn a_go_web_service_hears_the_answers_to_its_own_browser_calls() {
    let go = Command::new("go")
        .arg("version")
        .output()
        .is_ok_and(|output| output.status.success());
    if !WebApis::node_available() || !go {
        eprintln!("skipping: node or go is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Go, "go", WebApis::GO_PROMPTS).await;
    WebApis::assert_answered(root.path(), true);
}

#[tokio::test]
async fn a_csharp_web_service_hears_the_answers_to_its_own_browser_calls() {
    let workload = Command::new("dotnet")
        .args(["workload", "list"])
        .output()
        .is_ok_and(|output| String::from_utf8_lossy(&output.stdout).contains("wasm-tools"));
    if !WebApis::node_available() || !workload {
        eprintln!("skipping: node or the .NET wasm-tools workload is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = ("cs", WebApis::CSHARP_PROMPTS);
    let profile = crate::BuildProfile::Development;
    WebApis::publish_with(root.path(), WebLanguage::CSharp, source, profile).await;
    WebApis::assert_answered(root.path(), false);
}

#[tokio::test]
async fn a_dart_service_hears_the_answers_to_its_own_browser_calls() {
    if !WebApis::node_available() || !WebApis::dart_available() {
        eprintln!("skipping: node or dart is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish_dart(root.path(), WebApis::DART_PROMPTS).await;
    WebApis::assert_answered(root.path(), true);
}

#[tokio::test]
async fn a_ruby_web_service_hears_the_answers_to_its_own_browser_calls() {
    let unavailable = crate::tac_ruby::RubyRuntimeBuild::unavailable_reason();
    if !WebApis::node_available() || unavailable.is_some() {
        eprintln!("skipping: node or the Ruby runtime is unavailable: {unavailable:?}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Ruby, "rb", WebApis::RUBY_PROMPTS).await;
    WebApis::assert_answered(root.path(), true);
}

async fn publish_swift(root: &Path, source: &str) -> bool {
    if !super::swift_tests::swift_web_ready() {
        eprintln!("skipping: the Swift SDK for WebAssembly, swiftc or node is unavailable");
        return false;
    }
    let source = ("swift", source);
    let profile = crate::BuildProfile::Development;
    WebApis::publish_with(root, WebLanguage::Swift, source, profile).await;
    true
}

#[tokio::test]
async fn a_swift_web_service_reads_the_clock_and_secure_random_from_the_page() {
    let root = tempfile::tempdir().expect("project");
    if !publish_swift(root.path(), WebApis::SWIFT_SERVICE).await {
        return;
    }
    let driver = format!("{}{}", WebApis::FILE_FETCH, WebApis::SERVICE_DRIVER);
    let report = WebApis::run(root.path(), &driver);
    WebApis::assert_stamp(&report);
}

#[tokio::test]
async fn a_swift_web_service_reads_and_hears_connectivity_visibility_and_preferences() {
    let root = tempfile::tempdir().expect("project");
    if !publish_swift(root.path(), WebApis::SWIFT_LISTENER).await {
        return;
    }
    WebApis::assert_heard(root.path(), true);
}

#[tokio::test]
async fn a_swift_web_service_hears_the_answers_to_its_own_browser_calls() {
    let root = tempfile::tempdir().expect("project");
    if !publish_swift(root.path(), WebApis::SWIFT_PROMPTS).await {
        return;
    }
    WebApis::assert_answered(root.path(), true);
}

async fn publish_java(root: &Path, source: &str) -> bool {
    if !super::java_tests::java_web_ready() {
        eprintln!("skipping: TeaVM (TAC_TEAVM_HOME), javac or node is unavailable");
        return false;
    }
    let source = ("java", source);
    let profile = crate::BuildProfile::Development;
    WebApis::publish_with(root, WebLanguage::Java, source, profile).await;
    true
}

#[tokio::test]
async fn a_java_web_service_reads_the_clock_and_secure_random_from_the_page() {
    let root = tempfile::tempdir().expect("project");
    if !publish_java(root.path(), WebApis::JAVA_SERVICE).await {
        return;
    }
    let driver = format!("{}{}", WebApis::FILE_FETCH, WebApis::SERVICE_DRIVER);
    let report = WebApis::run(root.path(), &driver);
    WebApis::assert_stamp(&report);
}

#[tokio::test]
async fn a_java_web_service_reads_and_hears_connectivity_visibility_and_preferences() {
    let root = tempfile::tempdir().expect("project");
    if !publish_java(root.path(), WebApis::JAVA_LISTENER).await {
        return;
    }
    WebApis::assert_heard(root.path(), true);
}

#[tokio::test]
async fn a_java_web_service_hears_the_answers_to_its_own_browser_calls() {
    let root = tempfile::tempdir().expect("project");
    if !publish_java(root.path(), WebApis::JAVA_PROMPTS).await {
        return;
    }
    let driver = WebApis::PROMPTS_DRIVER.replace("stamp.notify()", "stamp.notifyPage()");
    WebApis::assert_answered_by(root.path(), true, &driver);
}
