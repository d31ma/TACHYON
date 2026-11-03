use super::{Page, RubyWebServices, write};

struct BrowserHttpPage;

impl BrowserHttpPage {
    const FILES: [(&'static str, &'static str); 6] = [
        (
            "client/models/Greeting.rb",
            "# @!attribute [r] message\n#   @return [String]\nGreeting = Data.define(:message)\n",
        ),
        (
            "client/remotes/GreetingClient.rb",
            "#[HTTPClient(\"/api\")]\n#[CacheSettings({\"name\":\"greetings\",\"ttlMs\":60000,\"maxBytes\":4096,\"maxEntries\":8})]\nclass GreetingClient\n  # @return [Greeting]\n  #[Cached]\n  #[GetRequest(\"/greeting\")]\n  def read\n  end\n\n  # @return [Greeting]\n  #[ClearCache({\"method\":\"read\"})]\n  #[PostRequest(\"/clear\")]\n  def clear\n  end\n\n  # @return [Array<Greeting>]\n  #[GetRequest(\"/greetings\")]\n  def all\n  end\nend\n",
        ),
        (
            "client/remotes/OverrideClient.rb",
            "#[HTTPClient(\"/api\")]\nclass OverrideClient\n  # @return [Greeting]\n  #[GetRequest(\"/never\")]\n  def read\n  end\nend\n",
        ),
        (
            "client/setups/GreetingSetup.rb",
            "#[Setup]\nclass GreetingSetup\n  # @param execution [BehaviorExecution]\n  # @return [Hash]\n  def self.configure(execution)\n    {}\n  end\nend\n",
        ),
        (
            "client/setups/OverrideSetup.rb",
            "#[Setup]\nclass OverrideSetup\n  # @param execution [BehaviorExecution]\n  # @return [Hash]\n  def self.configure(execution)\n    { http: { override: { exchange: ->(_request, _execution) { { \"message\" => \"Custom adapter wins\" } } } } }\n  end\nend\n",
        ),
        (
            "tac-storage.js",
            include_str!("../../template/tac-storage.js"),
        ),
    ];

    const SERVICE: &'static str = "#[Service]\nclass GreetingService\n  #[Needs]\n  # @return [GreetingClient]\n  attr_reader :greeting_client\n\n  #[Needs]\n  # @return [OverrideClient]\n  attr_reader :override_client\n\n  # @return [String]\n  attr_reader :message, :custom, :early\n  # @return [Integer]\n  attr_reader :loads, :clears, :bumps\n\n  def initialize\n    @message = \"\"\n    @custom = \"\"\n    @early = \"\"\n    @loads = 0\n    @clears = 0\n    @bumps = 0\n  end\n\n  #[OnStart]\n  def start\n    @early = early_code\n  end\n\n  def ask\n    @loads += 1\n    @message = greeting_client.read.message\n  end\n\n  def clear\n    greeting_client.clear\n    @clears += 1\n  end\n\n  def count_all\n    greeting_client.all.length\n  end\n\n  def ask_custom\n    @custom = override_client.read.message\n  end\n\n  def bump\n    @bumps += 1\n  end\n\n  def churn\n    kept = Array.new(2000) { |index| \"x\" * (index % 64) }\n    GC.start\n    kept.length\n  end\n\n  private\n\n  def early_code\n    greeting_client.all\n    \"\"\n  rescue BehaviorError => error\n    error.behavior_code\n  end\nend\n";

    const DRIVER: &'static str = "console.info = () => {}\n\
         globalThis.indexedDB = { databases: async () => [], open: () => ({}) }\n\
         ;(0, eval)(await readFile('./tac-storage.js', 'utf8'))\n\
         const started = [], cancelled = []\n\
         globalThis[Symbol.for('tachyon.tac.http')] = { start: (text) => {\n\
         \x20 const request = JSON.parse(text)\n\
         \x20 let settle\n\
         \x20 const answer = new Promise((resolve) => { settle = resolve })\n\
         \x20 started.push({ request, settle })\n\
         \x20 return { answer, cancel: () => { cancelled.push(request.path) } }\n\
         } }\n\
         const reply = (value) => started.at(-1).settle(JSON.stringify(value))\n\
         const outcome = (promise) => Promise.resolve(promise).then((value) => ({ value }), (error) => ({ code: error.code ?? '', message: String(error.message) }))\n\
         const { default: GreetingService } = await import('./GreetingService.js')\n\
         const one = new GreetingService({})\n\
         const early = one.early, keys = Object.keys(one)\n\
         const asking = one.ask()\n\
         const waiting = typeof asking?.then === 'function'\n\
         const bumped = one.bump(), churned = one.churn(), loadsWhileWaiting = one.loads\n\
         const first = started[0].request\n\
         reply({ value: { message: 'Hello from Yon' } })\n\
         await asking\n\
         const message = one.message\n\
         await one.ask()\n\
         const afterCache = started.length\n\
         const clearing = one.clear()\n\
         reply({ value: { message: 'Cleared' } })\n\
         await clearing\n\
         const revalidating = one.ask()\n\
         const revalidation = started.at(-1).request\n\
         reply({ value: { message: 'Fresh' } })\n\
         await revalidating\n\
         const wrong = outcome(one.countAll())\n\
         reply({ value: { message: 'not a list' } })\n\
         const failing = outcome(one.countAll())\n\
         reply({ error: { code: 'HTTP_STATUS' } })\n\
         const counting = outcome(one.countAll())\n\
         reply({ value: [{ message: 'a' }, { message: 'b' }] })\n\
         await one.askCustom()\n\
         const two = new GreetingService({})\n\
         const removed = outcome(two.countAll())\n\
         two.__tachyonDispose()\n\
         const [wrongShape, failed, counted, cancelledCall] = await Promise.all([wrong, failing, counting, removed])\n\
         console.log(JSON.stringify({ early, keys, waiting, bumped, churned, loadsWhileWaiting, first, message, afterCache, clears: one.clears, revalidation, wrongShape, failed, counted, custom: one.custom, cancelledCall, cancelled, paths: started.map((call) => call.request.path), finalMessage: one.message }))\n\
         process.exit(0)\n";

    async fn publish(root: &std::path::Path) {
        for (relative, text) in Self::FILES {
            write(root, relative, text);
        }
        Page::publish(root, &[("greeting", "GreetingService", Self::SERVICE)]).await;
    }
}

#[test]
fn a_needed_client_is_not_a_page_field() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    for (relative, text) in BrowserHttpPage::FILES {
        write(root.path(), relative, text);
    }
    let service = write(
        root.path(),
        "client/services/GreetingService.rb",
        BrowserHttpPage::SERVICE,
    );
    let program = RubyWebServices::program(
        root.path(),
        &[(
            String::from("greeting"),
            String::from("GreetingService"),
            service,
        )],
    )
    .unwrap_or_else(|failure| panic!("program: {failure}"));
    let registration = program
        .lines()
        .find(|line| line.starts_with("TacServices.register(\"greeting\", GreetingService, '"))
        .unwrap_or_else(|| panic!("registration: {program}"));
    assert!(
        registration.contains(
            "\"fields\":[\"message\",\"custom\",\"early\",\"loads\",\"clears\",\"bumps\"]"
        ),
        "{registration}"
    );
    assert!(
        program.contains("@greeting_client ||= GreetingClient.new(@tachyon_test_context)"),
        "{program}"
    );
    assert!(
        program.contains("TacValues.register_models('{\"Greeting\":"),
        "{program}"
    );
}

#[tokio::test]
async fn a_ruby_service_waits_for_the_browser_transport_without_blocking_the_page() {
    if let Some(reason) = Page::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    BrowserHttpPage::publish(root.path()).await;
    let report = Page::run(root.path(), BrowserHttpPage::DRIVER);
    assert_eq!(report["early"], "HTTP_REQUEST", "{report}");
    assert!(!report["keys"].to_string().contains("Client"), "{report}");
    assert_eq!(report["waiting"], true, "{report}");
    assert_eq!(report["bumped"], 1, "{report}");
    assert_eq!(report["churned"], 2000, "{report}");
    assert_eq!(report["loadsWhileWaiting"], 1, "{report}");
    let first = &report["first"];
    assert_eq!(first["method"], "GET", "{report}");
    assert_eq!(first["path"], "/api/greeting", "{report}");
    assert_eq!(first["credentials"], "omit", "{report}");
    assert_eq!(first["redirect"], "error", "{report}");
    assert_eq!(first["publicRead"], true, "{report}");
    assert_eq!(first["revalidate"], false, "{report}");
    assert!(first["deadline"].is_u64(), "{report}");
    assert!(first.get("signal").is_none(), "{report}");
    assert_eq!(report["message"], "Hello from Yon", "{report}");
    assert_eq!(report["afterCache"], 1, "{report}");
    assert_eq!(report["clears"], 1, "{report}");
    assert_eq!(report["revalidation"]["revalidate"], true, "{report}");
    assert_eq!(report["revalidation"]["publicRead"], true, "{report}");
    assert_eq!(report["wrongShape"]["code"], "HTTP_RESPONSE", "{report}");
    assert_eq!(report["failed"]["code"], "HTTP_STATUS", "{report}");
    assert_eq!(report["counted"]["value"], 2, "{report}");
    assert_eq!(report["custom"], "Custom adapter wins", "{report}");
    assert_eq!(report["cancelledCall"]["code"], "CANCELLED", "{report}");
    assert_eq!(
        report["cancelled"],
        serde_json::json!(["/api/greetings"]),
        "{report}"
    );
    assert!(
        !report["paths"].to_string().contains("/api/never"),
        "{report}"
    );
    assert_eq!(report["finalMessage"], "Fresh", "{report}");
}
