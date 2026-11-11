#![allow(clippy::expect_used)]

use super::WebLanguage;
use super::cpp_stream_tests::CppStreams;
use super::csharp_stream_tests::CsharpStreams;
use super::java_tests::java_web_ready;
use super::kotlin_tests::kotlin_web_ready;
use super::rust_marker_proof_tests::RustPage;
use super::socket_hooks_shared_tests::{SocketHooks, SocketHooksCase};
use super::swift_tests::swift_web_ready;

struct LanguageHooks;

impl LanguageHooks {
    const RUST: [(&'static str, &'static str); 2] = [
        (
            "client/models/ChatMessage.rs",
            "#[derive(Clone, Debug, Default)]\npub struct ChatMessage {\n    pub text: String,\n}\n",
        ),
        (
            "client/remotes/ChatClient.rs",
            "use crate::models::ChatMessage;\n\n#[HTTPClient(\"/chat\")]\ntrait ChatClient {\n    #[SocketRequest]\n    fn receive(&self, message: ChatMessage);\n}\n",
        ),
    ];

    const RUST_SERVICE: &'static str = "#[Service]\npub struct ChatService {\n    #[Needs]\n    chat: std::sync::Arc<ChatClient>,\n    pub opens: i64,\n    pub reopens: i64,\n    pub code: f64,\n    pub retrying: bool,\n    pub ticks: i64,\n}\n\n\
         impl ChatService {\n    #[OnEvent]\n    fn connected(&mut self, opened: SocketOpened) {\n        self.opens += 1;\n        self.reopens += i64::from(opened.reconnected);\n    }\n\n    \
         #[OnEvent]\n    fn disconnected(&mut self, closed: SocketClosed) {\n        self.code = closed.code;\n        self.retrying = closed.reconnecting;\n    }\n\n    \
         #[Every(\"30s\")]\n    pub fn refresh(&mut self) {\n        self.ticks += 1;\n    }\n\n    \
         pub async fn say(&mut self, text: String) -> BehaviorResult<()> {\n        self.chat.send(ChatMessage { text }).await\n    }\n}\n";

    const CSHARP: [(&'static str, &'static str); 2] = [
        (
            "client/models/ChatMessage.cs",
            "public record ChatMessage(string Text);\n",
        ),
        (
            "client/remotes/ChatClient.cs",
            "[HTTPClient(\"/chat\")]\npublic interface ChatClient {\n    [SocketRequest]\n    void Receive(Models.ChatMessage message);\n}\n",
        ),
    ];

    const CSHARP_SERVICE: &'static str = "using Models;\n\n[Service]\npublic sealed class ChatService {\n    [Needs]\n    private readonly ChatClient _chat;\n    public int Opens = 0;\n    public int Reopens = 0;\n    public double Code = 0;\n    public bool Retrying = false;\n    public int Ticks = 0;\n\n\
         [OnEvent]\n    public void Connected(SocketOpened opened) { Opens += 1; Reopens += opened.Reconnected ? 1 : 0; }\n\n\
         [OnEvent]\n    public void Disconnected(SocketClosed closed) { Code = closed.Code; Retrying = closed.Reconnecting; }\n\n\
         [Every(\"30s\")]\n    public void Refresh() { Ticks += 1; }\n\n\
         public System.Threading.Tasks.Task Say(string text) => _chat.Send(new ChatMessage(text));\n}\n";

    const FSHARP: [(&'static str, &'static str); 2] = [
        (
            "client/models/ChatMessage.fs",
            "type ChatMessage = { Text: string }\n",
        ),
        (
            "client/remotes/ChatClient.fs",
            "open Models\n\n[<HTTPClient(\"/chat\")>]\ntype ChatClient =\n    [<SocketRequest>]\n    abstract Receive: message: ChatMessage -> unit\n",
        ),
    ];

    const FSHARP_SERVICE: &'static str = "open System.Threading.Tasks\nopen Models\n\n[<Service>]\ntype ChatService() =\n    [<Needs>]\n    member val ChatClient : ChatClient = Unchecked.defaultof<ChatClient> with get, set\n\n    \
         member val Opens : int = 0 with get, set\n    member val Reopens : int = 0 with get, set\n    member val Code : float = 0.0 with get, set\n    member val Retrying : bool = false with get, set\n    member val Ticks : int = 0 with get, set\n\n    \
         [<OnEvent>]\n    member this.Connected(opened: SocketOpened) : unit =\n        this.Opens <- this.Opens + 1\n        this.Reopens <- this.Reopens + (if opened.Reconnected then 1 else 0)\n\n    \
         [<OnEvent>]\n    member this.Disconnected(closed: SocketClosed) : unit =\n        this.Code <- closed.Code\n        this.Retrying <- closed.Reconnecting\n\n    \
         [<Every(\"30s\")>]\n    member this.Refresh() : unit =\n        this.Ticks <- this.Ticks + 1\n\n    \
         member this.Say(text: string) : Task =\n        this.ChatClient.Send({ Text = text })\n";

    const SWIFT: [(&'static str, &'static str); 2] = [
        (
            "client/models/ChatMessage.swift",
            "struct ChatMessage {\n    let text: String\n}\n",
        ),
        (
            "client/remotes/ChatClient.swift",
            "@HTTPClient(\"/chat\")\nprotocol ChatClient {\n    @SocketRequest\n    func receive(_ message: ChatMessage)\n}\n",
        ),
    ];

    const SWIFT_SERVICE: &'static str = "@Service\nfinal class ChatService {\n    @Needs\n    private let chat: ChatClient\n    var opens: Int = 0\n    var reopens: Int = 0\n    var code: Double = 0\n    var retrying: Bool = false\n    var ticks: Int = 0\n\n    \
         @OnEvent\n    func connected(_ opened: SocketOpened) {\n        opens += 1\n        reopens += opened.reconnected ? 1 : 0\n    }\n\n    \
         @OnEvent\n    func disconnected(_ closed: SocketClosed) {\n        code = closed.code\n        retrying = closed.reconnecting\n    }\n\n    \
         @Every(\"30s\")\n    func refresh() {\n        ticks += 1\n    }\n\n    \
         func say(_ text: String) async throws {\n        try await chat.send(ChatMessage(text: text))\n    }\n}\n";

    const KOTLIN: [(&'static str, &'static str); 2] = [
        (
            "client/models/ChatMessage.kt",
            "data class ChatMessage(val text: String)\n",
        ),
        (
            "client/remotes/ChatClient.kt",
            "import models.ChatMessage\n\n@HTTPClient(\"/chat\")\ninterface ChatClient {\n    @SocketRequest\n    fun receive(message: ChatMessage)\n}\n",
        ),
    ];

    const KOTLIN_SERVICE: &'static str = "import models.ChatMessage\n\n@Service\nclass ChatService {\n    @Needs\n    private val chat: ChatClient\n    var opens: Int = 0\n    var reopens: Int = 0\n    var code: Double = 0.0\n    var retrying: Boolean = false\n    var ticks: Int = 0\n\n    \
         @OnEvent\n    fun connected(opened: SocketOpened) {\n        opens += 1\n        reopens += if (opened.reconnected) 1 else 0\n    }\n\n    \
         @OnEvent\n    fun disconnected(closed: SocketClosed) {\n        code = closed.code\n        retrying = closed.reconnecting\n    }\n\n    \
         @Every(\"30s\")\n    fun refresh() {\n        ticks += 1\n    }\n\n    \
         suspend fun say(text: String) {\n        chat.send(ChatMessage(text))\n    }\n}\n";

    const JAVA: [(&'static str, &'static str); 2] = [
        (
            "client/models/ChatMessage.java",
            "record ChatMessage(String text) {}\n",
        ),
        (
            "client/remotes/ChatClient.java",
            "import models.ChatMessage;\n\n@HTTPClient(\"/chat\")\ninterface ChatClient {\n    @SocketRequest\n    void receive(ChatMessage message);\n}\n",
        ),
    ];

    const JAVA_SERVICE: &'static str = "import models.ChatMessage;\n\n@Service\nclass ChatService {\n    @Needs\n    private final ChatClient chat;\n    int opens = 0;\n    int reopens = 0;\n    double code = 0;\n    boolean retrying = false;\n    int ticks = 0;\n\n    \
         @OnEvent\n    void connected(SocketOpened opened) {\n        opens += 1;\n        reopens += opened.reconnected() ? 1 : 0;\n    }\n\n    \
         @OnEvent\n    void disconnected(SocketClosed closed) {\n        code = closed.code();\n        retrying = closed.reconnecting();\n    }\n\n    \
         @Every(\"30s\")\n    void refresh() {\n        ticks += 1;\n    }\n\n    \
         void say(String text) {\n        chat.send(new ChatMessage(text));\n    }\n}\n";

    const CPP: [(&'static str, &'static str); 2] = [
        (
            "client/models/ChatMessage.hpp",
            "#pragma once\n#include <string>\n\nstruct ChatMessage {\n    std::string text;\n};\n",
        ),
        (
            "client/remotes/ChatClient.hpp",
            "#pragma once\n#include \"../models/ChatMessage.hpp\"\n\nclass [[HTTPClient(\"/chat\")]] ChatClient {\npublic:\n    [[SocketRequest]] void receive(const ChatMessage& message);\n};\n",
        ),
    ];

    const CPP_SERVICE: &'static str = "#pragma once\n#include <string>\n#include \"../remotes/ChatClient.hpp\"\n\n\
         class [[Service]] ChatService {\npublic:\n    [[Needs]] ChatClient& chat;\n    int opens = 0;\n    int reopens = 0;\n    double code = 0;\n    bool retrying = false;\n    int ticks = 0;\n\n    \
         [[OnEvent]] void connected(const SocketOpened& opened) { opens += 1; reopens += opened.reconnected ? 1 : 0; }\n\n    \
         [[OnEvent]] void disconnected(const SocketClosed& closed) { code = closed.code; retrying = closed.reconnecting; }\n\n    \
         [[Every(\"30s\")]] void refresh() { ticks += 1; }\n\n    \
         tachyon::Task<void> say(std::string text) { co_await chat.send(ChatMessage{text}); }\n};\n";

    async fn verify(ready: bool, case: &SocketHooksCase<'_>) {
        if !ready {
            eprintln!(
                "skipping: the {:?} toolchain or node is unavailable",
                case.language
            );
            return;
        }
        let root = tempfile::tempdir().expect("project");
        let report = SocketHooks::run(root.path(), case).await;
        SocketHooks::check(&report, case.sockets);
    }

    fn case(
        language: WebLanguage,
        extension: &'static str,
        files: &'static [(&'static str, &'static str)],
        service: &'static str,
    ) -> SocketHooksCase<'static> {
        SocketHooksCase {
            language,
            extension,
            files,
            service,
            sockets: true,
        }
    }
}

#[tokio::test]
async fn a_rust_service_hears_its_socket_and_runs_its_every_method() {
    let case = LanguageHooks::case(
        WebLanguage::Rust,
        "rs",
        &LanguageHooks::RUST,
        LanguageHooks::RUST_SERVICE,
    );
    LanguageHooks::verify(RustPage::ready(), &case).await;
}

#[tokio::test]
async fn a_csharp_service_hears_its_socket_and_runs_its_every_method() {
    let case = LanguageHooks::case(
        WebLanguage::CSharp,
        "cs",
        &LanguageHooks::CSHARP,
        LanguageHooks::CSHARP_SERVICE,
    );
    LanguageHooks::verify(CsharpStreams::tools_available(), &case).await;
}

#[tokio::test]
async fn an_fsharp_service_hears_its_socket_and_runs_its_every_method() {
    let case = LanguageHooks::case(
        WebLanguage::CSharp,
        "fs",
        &LanguageHooks::FSHARP,
        LanguageHooks::FSHARP_SERVICE,
    );
    LanguageHooks::verify(CsharpStreams::tools_available(), &case).await;
}

#[tokio::test]
async fn a_swift_service_hears_its_socket_and_runs_its_every_method() {
    let case = LanguageHooks::case(
        WebLanguage::Swift,
        "swift",
        &LanguageHooks::SWIFT,
        LanguageHooks::SWIFT_SERVICE,
    );
    LanguageHooks::verify(swift_web_ready(), &case).await;
}

#[tokio::test]
async fn a_kotlin_service_hears_its_socket_and_runs_its_every_method() {
    let case = LanguageHooks::case(
        WebLanguage::Kotlin,
        "kt",
        &LanguageHooks::KOTLIN,
        LanguageHooks::KOTLIN_SERVICE,
    );
    LanguageHooks::verify(kotlin_web_ready(), &case).await;
}

#[tokio::test]
async fn a_java_service_hears_its_socket_and_runs_its_every_method() {
    let case = LanguageHooks::case(
        WebLanguage::Java,
        "java",
        &LanguageHooks::JAVA,
        LanguageHooks::JAVA_SERVICE,
    );
    LanguageHooks::verify(java_web_ready(), &case).await;
}

#[tokio::test]
async fn a_cpp_service_hears_its_socket_and_runs_its_every_method() {
    let case = LanguageHooks::case(
        WebLanguage::Cpp,
        "hpp",
        &LanguageHooks::CPP,
        LanguageHooks::CPP_SERVICE,
    );
    LanguageHooks::verify(CppStreams::tools_available(), &case).await;
}

struct JavaScriptHooks;

impl JavaScriptHooks {
    const PATH: &'static str = "client/services/ChatService.js";
    const SERVICE: &'static str = "export default class ChatService {\n  opens = 0\n  reopens = 0\n  code = 0\n  retrying = false\n  ticks = 0\n\n  \
         /** @param {SocketOpened} opened */\n  @OnEvent()\n  connected(opened) {\n    this.opens += 1\n    this.reopens += opened.reconnected ? 1 : 0\n  }\n\n  \
         /** @param {SocketClosed} closed */\n  @OnEvent()\n  disconnected(closed) {\n    this.code = closed.code\n    this.retrying = closed.reconnecting\n  }\n\n  \
         @Every('30s')\n  refresh() {\n    this.ticks += 1\n  }\n}\n";
}

#[test]
fn a_javascript_service_hears_its_socket_and_runs_its_every_method() {
    if !crate::node_driver::NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let path = std::path::Path::new(JavaScriptHooks::PATH);
    let module = crate::events::extract(path, JavaScriptHooks::SERVICE, "tac")
        .expect("events")
        .expect("module");
    let mut plan = serde_json::to_value(&module.types[0]).expect("plan");
    plan["streams"] = serde_json::json!([{ "method": "receive", "path": "/chat", "event": "ChatMessage", "socket": true }]);
    let root = tempfile::tempdir().expect("project");
    let page = format!(
        "{}\nexport const __tachyon_event_plan__ = {}\n",
        module.source,
        serde_json::json!({ "types": [plan] })
    );
    SocketHooks::write(root.path(), "ChatService.js", page.as_bytes());
    let report = SocketHooks::drive(root.path(), true);
    SocketHooks::check(&report, true);
}
