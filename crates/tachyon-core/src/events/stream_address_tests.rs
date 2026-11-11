use super::{EventModule, extract};
use crate::Failure;
use std::path::Path;

struct StreamAddress;

impl StreamAddress {
    const MANIFEST: &'static str = r#"{"tac":{"yon":{"origins":{"realtime":"https://rt.example.com"}},"outgoing":{"hosts":["rt.example.com"]}}}"#;
    const CLIENT: &'static str = r#"{"path": "/chat", "origin": "realtime"}"#;
    const MALFORMED: &'static str = r#"{"path": "/chat", "origin": "Real Time"}"#;
    const SOCKET: &'static str =
        r#"{"path": "/ws", "reconnect": {"maxTries": 0, "delayMs": 500, "maxDelayMs": 30000}}"#;
    const ADDRESS: &'static str =
        "/.tachyon/reconnect/0/500/30000/1000/.tachyon/origin/realtime/chat/ws";
    const CLIENTS: [(&'static str, &'static str); 11] = [
        (
            "js",
            "/**\n * @typedef {Object} ChatMessage\n * @property {string} text\n */\n@HTTPClient(CLIENT)\nexport class ChatClient {\n  /** @param {ChatMessage} message */\n  @SocketRequest(SOCKET)\n  receive(message) {}\n}\n",
        ),
        (
            "ts",
            "@HTTPClient(CLIENT)\nexport class ChatClient {\n  @SocketRequest(SOCKET)\n  receive(message: ChatMessage) {}\n}\n",
        ),
        (
            "go",
            "// @HTTPClient(CLIENT)\ntype ChatClient interface {\n\t// @SocketRequest(SOCKET)\n\tReceive(message ChatMessage)\n}\n",
        ),
        (
            "rb",
            "#[HTTPClient(CLIENT)]\nclass ChatClient\n  # @param message [ChatMessage]\n  #[SocketRequest(SOCKET)]\n  def receive(message)\n  end\nend\n",
        ),
        (
            "rs",
            "use crate::models::ChatMessage;\n\n#[HTTPClient(CLIENT)]\ntrait ChatClient {\n    #[SocketRequest(SOCKET)]\n    fn receive(&self, message: ChatMessage);\n}\n",
        ),
        (
            "dart",
            "@HTTPClient(CLIENT)\nabstract interface class ChatClient {\n  @SocketRequest(SOCKET)\n  void receive(ChatMessage message);\n}\n",
        ),
        (
            "swift",
            "@HTTPClient(CLIENT)\nprotocol ChatClient {\n    @SocketRequest(SOCKET)\n    func receive(_ message: ChatMessage)\n}\n",
        ),
        (
            "kt",
            "import models.ChatMessage\n\n@HTTPClient(CLIENT)\ninterface ChatClient {\n    @SocketRequest(SOCKET)\n    fun receive(message: ChatMessage)\n}\n",
        ),
        (
            "java",
            "import models.ChatMessage;\n\n@HTTPClient(CLIENT)\ninterface ChatClient {\n    @SocketRequest(SOCKET)\n    void receive(ChatMessage message);\n}\n",
        ),
        (
            "cs",
            "[HTTPClient(CLIENT)]\npublic interface ChatClient {\n    [SocketRequest(SOCKET)]\n    void Receive(Models.ChatMessage message);\n}\n",
        ),
        (
            "py",
            "from typing import Protocol\nfrom models import ChatMessage\n\n@http_client(CLIENT)\nclass ChatClient(Protocol):\n    @socket_request(SOCKET)\n    def receive(self, message: ChatMessage) -> None: ...\n",
        ),
    ];

    fn project() -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        std::fs::create_dir_all(root.path().join("client/remotes")).expect("remotes");
        std::fs::write(root.path().join("client/manifest.json"), Self::MANIFEST).expect("manifest");
        root
    }

    fn extracted(
        root: &Path,
        extension: &str,
        template: &str,
        client: &str,
    ) -> Result<Option<EventModule>, Failure> {
        let source = template
            .replace("CLIENT", client)
            .replace("SOCKET", Self::SOCKET);
        let path = root.join(format!("client/remotes/ChatClient.{extension}"));
        std::fs::write(&path, &source).expect("client");
        extract(&path, &source, "tac")
    }

    fn address(root: &Path, extension: &str, template: &str) -> String {
        let module = Self::extracted(root, extension, template, Self::CLIENT)
            .unwrap_or_else(|failure| panic!("{extension}: {failure}"))
            .unwrap_or_else(|| panic!("{extension}: no event module"));
        module
            .types
            .iter()
            .flat_map(|event_type| &event_type.streams)
            .map(|stream| stream.path.clone())
            .next()
            .unwrap_or_else(|| panic!("{extension}: no stream"))
    }
}

#[test]
fn every_client_language_writes_its_origin_and_reconnect_policy_into_the_socket_address() {
    let root = StreamAddress::project();
    for (extension, template) in StreamAddress::CLIENTS {
        assert_eq!(
            StreamAddress::address(root.path(), extension, template),
            StreamAddress::ADDRESS,
            "{extension}"
        );
    }
}

#[test]
fn every_client_language_refuses_an_origin_that_is_not_a_plain_name() {
    let root = StreamAddress::project();
    for (extension, template) in StreamAddress::CLIENTS {
        let refused =
            StreamAddress::extracted(root.path(), extension, template, StreamAddress::MALFORMED)
                .expect_err(extension)
                .to_string();
        assert!(
            refused.contains("is not a name from tac.yon.origins"),
            "{extension}: {refused}"
        );
    }
}

#[test]
fn a_stream_request_refuses_a_policy_outside_its_limits_and_a_reserved_path() {
    let root = StreamAddress::project();
    let template = StreamAddress::CLIENTS[1].1;
    for socket in [
        r#"{"reconnect": {"maxTries": 101}}"#,
        r#"{"reconnect": {"maxTries": 0, "jitterMs": 0}}"#,
        r#"{"reconnect": {"delayMs": 5000, "maxDelayMs": 1000}}"#,
        r#"{"reconnect": {"delayMs": 10}}"#,
        r#"{"reconnect": {"tries": 3}}"#,
        r#"{"reconnect": 3}"#,
        r#"{"path": "/ws", "retry": {}}"#,
        r#""/.tachyon/origin/realtime/ws""#,
        r#""/.tachyon/reconnect/0/1/1/1/ws""#,
    ] {
        let source = template.replace("SOCKET", socket);
        let refused = StreamAddress::extracted(root.path(), "ts", &source, "\"/chat\"")
            .expect_err(socket)
            .to_string();
        assert!(
            refused.contains("@SocketRequest") || refused.contains("/.tachyon/"),
            "{socket}: {refused}"
        );
    }
}

#[test]
fn a_stream_request_without_a_policy_keeps_its_plain_address() {
    let root = StreamAddress::project();
    let template = StreamAddress::CLIENTS[1].1.replace("SOCKET", "\"/ws\"");
    let module = StreamAddress::extracted(root.path(), "ts", &template, "\"/chat\"")
        .expect("plain")
        .expect("module");
    assert_eq!(module.types[0].streams[0].path, "/chat/ws");
}

#[test]
fn http_requests_of_a_named_origin_client_carry_the_origin_in_their_path() {
    let root = StreamAddress::project();
    for (extension, source) in [
        (
            "ts",
            "@HTTPClient(CLIENT)\nexport class ChatClient {\n  @GetRequest(\"/rooms\")\n  rooms(): Promise<Room[]> {}\n}\n",
        ),
        (
            "java",
            "@HTTPClient(CLIENT)\ninterface ChatClient {\n    @GetRequest(\"/rooms\")\n    List<Room> rooms();\n}\n",
        ),
        (
            "py",
            "from typing import Protocol\n\n@http_client(CLIENT)\nclass ChatClient(Protocol):\n    @get_request(\"/rooms\")\n    def rooms(self) -> list[Room]: ...\n",
        ),
    ] {
        let source = source.replace("CLIENT", StreamAddress::CLIENT);
        let path = root
            .path()
            .join(format!("client/remotes/ChatClient.{extension}"));
        let lowered = crate::behavior::HttpClientSource::rewrite(&path, &source)
            .unwrap_or_else(|failure| panic!("{extension}: {failure}"))
            .expect("lowered");
        assert!(
            lowered.contains("\"path\":\"/.tachyon/origin/realtime/chat/rooms\""),
            "{extension}: {lowered}"
        );
    }
}
