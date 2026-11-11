#![allow(clippy::expect_used)]

use super::dart_stream_tests::DartStreams;
use crate::tac_web::socket_hooks_shared_tests::SocketHooks;

struct DartHooks;

impl DartHooks {
    const FILES: [(&'static str, &'static str); 3] = [
        (
            "client/models/ChatMessage.dart",
            "class ChatMessage {\n  final String text;\n\n  const ChatMessage(this.text);\n}\n",
        ),
        (
            "client/remotes/ChatClient.dart",
            "@HTTPClient(\"/chat\")\nabstract interface class ChatClient {\n  @SocketRequest\n  void receive(ChatMessage message);\n}\n",
        ),
        (
            "client/services/ChatService.dart",
            "@Service\nclass ChatService {\n  @Needs\n  final ChatClient chatClient;\n  int opens = 0;\n  int reopens = 0;\n  double code = 0;\n  bool retrying = false;\n  int ticks = 0;\n\n\
             \x20 @OnEvent\n  void connected(SocketOpened opened) {\n    opens += 1;\n    reopens += opened.reconnected ? 1 : 0;\n  }\n\n\
             \x20 @OnEvent\n  void disconnected(SocketClosed closed) {\n    code = closed.code;\n    retrying = closed.reconnecting;\n  }\n\n\
             \x20 @Every('30s')\n  void refresh() {\n    ticks += 1;\n  }\n\n\
             \x20 Future<void> say(String text) => chatClient.send(ChatMessage(text));\n}\n",
        ),
    ];

    const SERVICES: [(&'static str, &'static str); 1] =
        [("ChatService", "client/services/ChatService.dart")];
}

#[tokio::test]
async fn a_dart_service_hears_its_socket_and_runs_its_every_method() {
    if !DartStreams::tools_available() {
        eprintln!("skipping: dart or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    DartStreams::publish(
        root.path(),
        &DartHooks::FILES,
        &DartHooks::SERVICES,
        &["chat"],
    )
    .await;
    let report = DartStreams::drive(
        root.path(),
        &SocketHooks::driver(true, "application-events.js"),
    );
    SocketHooks::check(&report, true);
}
