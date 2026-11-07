use super::{Page, RubyWebServices, write};
use std::path::{Path, PathBuf};

struct StreamPage;

impl StreamPage {
    const FILES: [(&'static str, &'static str); 6] = [
        (
            "client/models/Tick.rb",
            "# @!attribute [r] level\n#   @return [Integer]\nTick = Data.define(:level)\n",
        ),
        (
            "client/models/ChatMessage.rb",
            "# @!attribute [r] text\n#   @return [String]\nChatMessage = Data.define(:text)\n",
        ),
        (
            "client/remotes/TicksClient.rb",
            "#[HTTPClient(\"/ticks\")]\nclass TicksClient\n  # @param tick [Tick]\n  #[StreamRequest]\n  def receive(tick)\n  end\nend\n",
        ),
        (
            "client/remotes/ChatClient.rb",
            "#[HTTPClient(\"/chat\")]\nclass ChatClient\n  # @param message [ChatMessage]\n  #[SocketRequest]\n  def receive(message)\n  end\nend\n",
        ),
        (
            "client/delegates/LevelDelegate.rb",
            "#[Delegate]\nclass LevelDelegate\n  #[Needs]\n  # @return [ScaleDelegate]\n  attr_reader :scale\n\n  def raised(level)\n    scale.times(level) + 1\n  end\nend\n",
        ),
        (
            "client/delegates/ScaleDelegate.rb",
            "#[Delegate]\nclass ScaleDelegate\n  def times(level)\n    level * 10\n  end\nend\n",
        ),
    ];

    const SERVICE: &'static str = "#[Service]\nclass TicksService\n  #[Needs]\n  # @return [TicksClient]\n  attr_reader :ticks\n\n  #[Needs]\n  # @return [ChatClient]\n  attr_reader :chat\n\n  #[Needs]\n  # @return [LevelDelegate]\n  attr_reader :levels\n\n  # @return [Integer]\n  attr_reader :level\n  # @return [String]\n  attr_reader :said\n\n  def initialize\n    @level = 0\n    @said = \"\"\n  end\n\n  # @param tick [Tick]\n  #[OnEvent]\n  def heard(tick)\n    @level = levels.raised(tick.level)\n  end\n\n  # @param message [ChatMessage]\n  #[OnEvent]\n  def answered(message)\n    @said = message.text\n  end\n\n  def say(text)\n    chat.send_message(ChatMessage.new(text: text))\n  end\n\n  def say_too_much\n    chat.send_message(ChatMessage.new(text: \"x\" * 70_000))\n  rescue BehaviorError => error\n    error.behavior_code\n  end\nend\n";

    const DRIVER: &'static str = "globalThis.location = { href: 'http://127.0.0.1:4000/page' }\n\
         const opened = []\n\
         globalThis.EventSource = class { constructor(url) { this.url = String(url); this.closed = false; opened.push(this) } addEventListener() {} close() { this.closed = true } }\n\
         globalThis.WebSocket = class { constructor(url) { this.url = String(url); this.readyState = 0; this.sent = []; this.closed = false; opened.push(this) } send(text) { this.sent.push(text) } close() { this.closed = true } }\n\
         const { default: TicksService, __tachyon_event_plan__: plan } = await import('./TicksService.js')\n\
         const { default: ApplicationEvents } = await import('./application-events.mjs')\n\
         let failures = 0\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => { failures += 1 } })\n\
         const ticks = new TicksService({})\n\
         bus.attach(ticks, plan.types[0], { applicationEventPublisher: null })\n\
         await bus.initialize(ticks)\n\
         const stream = opened.find((source) => !('sent' in source)), socket = opened.find((source) => 'sent' in source)\n\
         const queued = [ticks.say('early')]\n\
         for (let index = 0; index < 15; index += 1) queued.push(ticks.say(`queued ${index}`))\n\
         let refused = ''\n\
         try { await ticks.say('overflow') } catch (error) { refused = error.code }\n\
         socket.readyState = 1\n\
         socket.onopen()\n\
         await Promise.all(queued)\n\
         stream.onmessage({ data: JSON.stringify({ level: 3 }), lastEventId: '' })\n\
         stream.onmessage({ data: JSON.stringify({ level: 'high' }), lastEventId: '' })\n\
         socket.onmessage({ data: JSON.stringify({ text: 'hello' }) })\n\
         await ticks.say('late')\n\
         const tooMuch = await ticks.sayTooMuch()\n\
         const report = { urls: opened.map((source) => source.url).sort(), level: ticks.level, said: ticks.said, failures, sent: socket.sent, tooMuch, refused }\n\
         bus.dispose(ticks)\n\
         await new Promise((resolve) => setTimeout(resolve, 50))\n\
         report.closed = opened.map((source) => source.closed)\n\
         console.log(JSON.stringify(report))\n\
         process.exit(0)\n";

    fn write_files(root: &Path) {
        for (relative, text) in Self::FILES {
            write(root, relative, text);
        }
    }

    fn program(root: &Path, service: &str) -> Result<String, crate::Failure> {
        let path: PathBuf = write(root, "client/services/TicksService.rb", service);
        RubyWebServices::program(
            root,
            &[(String::from("ticks"), String::from("TicksService"), path)],
        )
    }

    async fn publish(root: &Path) {
        Self::write_files(root);
        Page::publish(root, &[("ticks", "TicksService", Self::SERVICE)]).await;
        write(
            root,
            "application-events.mjs",
            concat!(
                include_str!("../../template/application-events.js"),
                "\nexport default TacApplicationEvents\n"
            ),
        );
    }
}

#[test]
fn a_ruby_service_describes_the_streams_of_the_clients_it_needs() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    StreamPage::write_files(root.path());
    let program = StreamPage::program(root.path(), StreamPage::SERVICE)
        .unwrap_or_else(|failure| panic!("program: {failure}"));
    let registration = program
        .lines()
        .find(|line| line.starts_with("TacServices.register(\"ticks\", TicksService, '"))
        .unwrap_or_else(|| panic!("registration: {program}"));
    for expected in [
        "{\"client\":\"ChatClient\",\"event\":\"ChatMessage\"",
        "\"path\":\"/chat\",\"socket\":true}",
        "{\"client\":\"TicksClient\",\"event\":\"Tick\"",
        "\"path\":\"/ticks\",\"socket\":false}",
    ] {
        assert!(
            registration.contains(expected),
            "{expected}: {registration}"
        );
    }
    for expected in [
        "@levels ||= LevelDelegate.new",
        "@scale ||= ScaleDelegate.new",
        "BehaviorBrowserSocket.send_message(\"ChatClient\", message)",
        "def initialize(_context = nil)",
    ] {
        assert!(program.contains(expected), "{expected}: {program}");
    }
    assert!(!program.contains("Behaviors.create(ChatSetup"), "{program}");
    assert!(
        !program.contains("Behaviors.create(TicksSetup"),
        "{program}"
    );
}

#[test]
fn a_service_that_does_not_need_a_stream_client_holds_no_stream() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    StreamPage::write_files(root.path());
    let quiet = "#[Service]\nclass TicksService\n  #[Needs]\n  # @return [ScaleDelegate]\n  attr_reader :scale\n\n  # @param tick [Tick]\n  #[OnEvent]\n  def heard(tick)\n    scale.times(tick.level)\n  end\nend\n";
    let program = StreamPage::program(root.path(), quiet)
        .unwrap_or_else(|failure| panic!("program: {failure}"));
    assert!(program.contains("\"streams\":[]"), "{program}");
}

#[test]
fn a_ruby_helper_with_a_lifecycle_hook_is_refused() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    StreamPage::write_files(root.path());
    write(
        root.path(),
        "client/delegates/ScaleDelegate.rb",
        "#[Delegate]\nclass ScaleDelegate\n  #[OnStart]\n  def start\n    @ready = true\n  end\n\n  def times(level)\n    level * 10\n  end\nend\n",
    );
    let failure = StreamPage::program(root.path(), StreamPage::SERVICE)
        .expect_err("a Delegate hook the page never runs");
    let text = failure.to_string();
    assert!(
        text.contains("The Ruby Delegate ScaleDelegate has an OnStart"),
        "{text}"
    );
    assert!(text.contains("client/delegates/ScaleDelegate.rb"), "{text}");
}

#[tokio::test]
async fn a_ruby_service_receives_streams_and_sends_on_its_socket() {
    if let Some(reason) = Page::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    StreamPage::publish(root.path()).await;
    let report = Page::run(root.path(), StreamPage::DRIVER);
    assert_eq!(
        report["urls"],
        serde_json::json!(["/ticks", "ws://127.0.0.1:4000/chat"]),
        "{report}"
    );
    assert_eq!(report["level"], 31, "{report}");
    assert_eq!(report["said"], "hello", "{report}");
    assert_eq!(report["failures"], 1, "{report}");
    assert_eq!(report["refused"], "HTTP_NETWORK", "{report}");
    let sent = report["sent"].as_array().expect("sent");
    assert_eq!(sent.len(), 17, "{report}");
    assert_eq!(sent[0], "{\"text\":\"early\"}", "{report}");
    assert_eq!(sent[15], "{\"text\":\"queued 14\"}", "{report}");
    assert_eq!(sent[16], "{\"text\":\"late\"}", "{report}");
    assert_eq!(report["tooMuch"], "HTTP_BODY", "{report}");
    assert_eq!(
        report["closed"],
        serde_json::json!([true, true]),
        "{report}"
    );
}
