use super::{LuaWebServices, Page};
use std::path::{Path, PathBuf};

struct StreamPage;

impl StreamPage {
    const FILES: [(&'static str, &'static str); 4] = [
        (
            "client/models/Tick.lua",
            "---@class Tick\n---@field level integer\nTick = {}\n",
        ),
        (
            "client/models/ChatMessage.lua",
            "---@class ChatMessage\n---@field text string\nChatMessage = {}\n",
        ),
        (
            "client/remotes/TicksClient.lua",
            "-- @HTTPClient({\"path\":\"/ticks\"})\n---@class TicksClient\nTicksClient = {}\n\n---@param tick Tick\n---@return nil\n-- @StreamRequest\nfunction TicksClient:receive(tick)\nend\n",
        ),
        (
            "client/remotes/ChatClient.lua",
            "-- @HTTPClient({\"path\":\"/chat\"})\n---@class ChatClient\nChatClient = {}\n\n---@param message ChatMessage\n---@return nil\n-- @SocketRequest\nfunction ChatClient:receive(message)\nend\n",
        ),
    ];

    const SERVICE: &'static str = "-- @Service\n---@class TicksService\nTicksService = {\n  -- @Needs\n  ---@type TicksClient\n  ticks = nil,\n  -- @Needs\n  ---@type ChatClient\n  chat = nil,\n  ---@type integer\n  level = 0,\n  ---@type string\n  said = \"\",\n}\n\n---@param tick Tick\n---@return nil\n-- @OnEvent\nfunction TicksService:heard(tick)\n  self.level = tick.level * 10 + 1\nend\n\n---@param message ChatMessage\n---@return nil\n-- @OnEvent\nfunction TicksService:answered(message)\n  self.said = message.text\nend\n\n---@param text string\n---@return nil\nfunction TicksService:say(text)\n  self.chat:send(setmetatable({ text = text }, ChatMessage))\nend\n\n---@return string\nfunction TicksService:say_too_much()\n  local _, failure = pcall(self.chat.send, self.chat, setmetatable({ text = string.rep(\"x\", 70000) }, ChatMessage))\n  return failure.behavior_code\nend\n";

    const UNIT: [(&'static str, &'static str); 2] = [
        (
            "client/tests/StreamTestSetup.lua",
            "-- @TestSetup\n---@class StreamTestSetup\nStreamTestSetup = {}\n\n---@param execution table\n---@return table\nfunction StreamTestSetup.configure(execution)\n  return {}\nend\n",
        ),
        (
            "client/tests/ChatClientTest.lua",
            "-- @Test(StreamTestSetup)\n---@class ChatClientTest\nChatClientTest = {\n  -- @Fake\n  ---@type ChatClient\n  chat = nil,\n}\n\nfunction ChatClientTest:test_send_has_no_page_to_send_on()\n  local chat = TacServices.construct(ChatClient)\n  local ok, failure = pcall(chat.send, chat, setmetatable({ text = \"hello\" }, ChatMessage))\n  Assert.equals(false, ok)\n  Assert.equals(\"HTTP_MISSING\", failure.behavior_code)\nend\n",
        ),
    ];

    const SITE_FILES: [(&'static str, &'static str); 6] = [
        (
            "client/models/Tick.lua",
            include_str!("../../../../../website/content/features/realtime/client/models/Tick.lua"),
        ),
        (
            "client/remotes/TicksClient.lua",
            include_str!(
                "../../../../../website/content/features/realtime/client/remotes/TicksClient.lua"
            ),
        ),
        (
            "client/services/StreamService.lua",
            include_str!(
                "../../../../../website/content/features/realtime/client/services/StreamService.lua"
            ),
        ),
        (
            "client/models/ChatMessage.lua",
            include_str!(
                "../../../../../website/content/features/sockets/client/models/ChatMessage.lua"
            ),
        ),
        (
            "client/remotes/ChatClient.lua",
            include_str!(
                "../../../../../website/content/features/sockets/client/remotes/ChatClient.lua"
            ),
        ),
        (
            "client/services/ChatService.lua",
            include_str!(
                "../../../../../website/content/features/sockets/client/services/ChatService.lua"
            ),
        ),
    ];

    const QUIET: &'static str = "-- @Service\n---@class TicksService\nTicksService = {\n  ---@type integer\n  level = 0,\n}\n\n---@param tick Tick\n---@return nil\n-- @OnEvent\nfunction TicksService:heard(tick)\n  self.level = tick.level\nend\n";

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
            Page::write(root, relative, text);
        }
    }

    fn program(root: &Path, service: &str) -> String {
        let path: PathBuf = Page::write(root, "client/services/TicksService.lua", service);
        LuaWebServices::program(
            root,
            &[(String::from("ticks"), String::from("TicksService"), path)],
        )
        .unwrap_or_else(|failure| panic!("program: {failure}"))
    }

    fn registration(program: &str) -> &str {
        program
            .lines()
            .find(|line| line.starts_with("TacServices.register([[ticks]], TicksService, "))
            .unwrap_or_else(|| panic!("registration: {program}"))
    }
}

#[test]
fn a_lua_service_describes_the_streams_of_the_clients_it_needs() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    StreamPage::write_files(root.path());
    let program = StreamPage::program(root.path(), StreamPage::SERVICE);
    let registration = StreamPage::registration(&program);
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
        "function ChatClient:send(message)",
        "BehaviorBrowserSocket.send(\"ChatClient\", message)",
        "function BehaviorBrowserSocket.send(client, message)",
    ] {
        assert!(program.contains(expected), "{expected}: {program}");
    }
    assert!(!program.contains("function TicksClient:send"), "{program}");
    assert!(!program.contains("Behaviors.create(ChatSetup"), "{program}");
    assert!(
        !program.contains("Behaviors.create(TicksSetup"),
        "{program}"
    );
}

#[test]
fn a_lua_service_that_does_not_need_a_stream_client_holds_no_stream() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    StreamPage::write_files(root.path());
    let program = StreamPage::program(root.path(), StreamPage::QUIET);
    let registration = StreamPage::registration(&program);
    assert!(registration.contains("\"streams\":[]"), "{registration}");
    assert!(
        !program.contains("function BehaviorBrowserSocket.send"),
        "{program}"
    );
}

#[tokio::test]
async fn a_lua_service_receives_streams_and_sends_on_its_socket() {
    if let Some(reason) = Page::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    StreamPage::write_files(root.path());
    Page::publish(
        root.path(),
        &[("ticks", "TicksService", StreamPage::SERVICE)],
    )
    .await;
    Page::event_runtime(root.path());
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
    let sent = report["sent"]
        .as_array()
        .unwrap_or_else(|| panic!("sent: {report}"));
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

#[test]
fn a_lua_test_unit_has_no_page_to_send_a_socket_message_on() {
    if crate::lua_source::LuaProgram::new().is_none() {
        eprintln!("skipping: lua is unavailable");
        return;
    }
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    StreamPage::write_files(root.path());
    for (relative, text) in StreamPage::UNIT {
        Page::write(root.path(), relative, text);
    }
    let report =
        crate::client_tests::run(root.path(), Path::new("client/tests/ChatClientTest.lua"))
            .unwrap_or_else(|failure| panic!("{failure}"));
    assert_eq!(report.cases.len(), 1);
    for case in &report.cases {
        assert!(case.passed, "{}: {:?}", case.name, case.error);
    }
}

#[test]
fn the_website_lua_stream_and_socket_samples_build() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    for (relative, text) in StreamPage::SITE_FILES {
        crate::annotation_layout::validate(Path::new(relative), text)
            .unwrap_or_else(|failure| panic!("{relative}: {failure}"));
        Page::write(root.path(), relative, text);
    }
    for (class, expected) in [
        ("StreamService", "\"path\":\"/ticks\",\"socket\":false}"),
        ("ChatService", "\"path\":\"/chat\",\"socket\":true}"),
    ] {
        let path = root.path().join(format!("client/services/{class}.lua"));
        let program = LuaWebServices::program(
            root.path(),
            &[(String::from("site"), String::from(class), path)],
        )
        .unwrap_or_else(|failure| panic!("{class}: {failure}"));
        assert!(program.contains(expected), "{class}: {program}");
    }
}
