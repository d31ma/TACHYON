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

    const STREAM: &'static str = r#"{"path": "/state", "event": "state", "reconnect": {"maxTries": 0, "delayMs": 500, "maxDelayMs": 30000}}"#;
    const QUERIED: &'static str =
        r#"{"path": "/stats/{window}", "event": "state", "query": "StatsWindow"}"#;
    const QUERIED_ADDRESS: &'static str = "/.tachyon/event/state/.tachyon/query/StatsWindow/.tachyon/origin/realtime/chat/stats/{window}";
    const NAMED_ADDRESS: &'static str = "/.tachyon/reconnect/0/500/30000/1000/.tachyon/event/state/.tachyon/origin/realtime/chat/state";

    fn stream_template(template: &str, arguments: &str) -> String {
        template
            .replace("SocketRequest", "StreamRequest")
            .replace("socket_request", "stream_request")
            .replace("SOCKET", arguments)
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

#[test]
fn every_client_language_writes_a_stream_event_name_into_the_stream_address() {
    let root = StreamAddress::project();
    for (extension, template) in StreamAddress::CLIENTS {
        let source = StreamAddress::stream_template(template, StreamAddress::STREAM);
        assert_eq!(
            StreamAddress::address(root.path(), extension, &source),
            StreamAddress::NAMED_ADDRESS,
            "{extension}"
        );
    }
}

#[test]
fn a_stream_event_name_must_be_a_plain_name_that_tachyon_does_not_read_itself() {
    let root = StreamAddress::project();
    let template = StreamAddress::CLIENTS[1].1;
    for (arguments, said) in [
        (r#"{"event": "message"}"#, "Tachyon reads those itself"),
        (r#"{"event": "heartbeat"}"#, "Tachyon reads those itself"),
        (r#"{"event": "end"}"#, "Tachyon reads those itself"),
        (r#"{"event": "error"}"#, "Tachyon reads those itself"),
        (r#"{"event": "1state"}"#, "starts with a letter"),
        (r#"{"event": "state/now"}"#, "starts with a letter"),
        (r#"{"event": ""}"#, "starts with a letter"),
        (r#"{"event": 3}"#, "starts with a letter"),
        (r#""/.tachyon/event/state/ticks""#, "/.tachyon/event/"),
    ] {
        let source = StreamAddress::stream_template(template, arguments);
        let refused = StreamAddress::extracted(root.path(), "ts", &source, "\"/chat\"")
            .expect_err(arguments)
            .to_string();
        assert!(refused.contains(said), "{arguments}: {refused}");
    }
    let long = format!(r#"{{"event": "{}"}}"#, "a".repeat(65));
    let source = StreamAddress::stream_template(template, &long);
    let refused = StreamAddress::extracted(root.path(), "ts", &source, "\"/chat\"")
        .expect_err("long")
        .to_string();
    assert!(refused.contains("at most 64"), "{refused}");
}

#[test]
fn a_socket_request_refuses_an_event_name() {
    let root = StreamAddress::project();
    for (extension, template) in StreamAddress::CLIENTS {
        let source = template.replace("SOCKET", r#"{"path": "/ws", "event": "state"}"#);
        let refused = StreamAddress::extracted(root.path(), extension, &source, "\"/chat\"")
            .expect_err(extension)
            .to_string();
        assert!(
            refused.contains("@SocketRequest takes no event name"),
            "{extension}: {refused}"
        );
    }
}

#[test]
fn a_stream_event_name_keeps_dots_dashes_and_underscores() {
    let root = StreamAddress::project();
    let template = StreamAddress::CLIENTS[1].1;
    let source =
        StreamAddress::stream_template(template, r#"{"path": "/jobs", "event": "Job.state-v2_x"}"#);
    let module = StreamAddress::extracted(root.path(), "ts", &source, "\"/chat\"")
        .expect("named")
        .expect("module");
    assert_eq!(
        module.types[0].streams[0].path,
        "/.tachyon/event/Job.state-v2_x/chat/jobs"
    );
}

#[test]
fn every_client_language_writes_a_stream_query_model_into_the_stream_address() {
    let root = StreamAddress::project();
    for (extension, template) in StreamAddress::CLIENTS {
        let source = StreamAddress::stream_template(template, StreamAddress::QUERIED);
        assert_eq!(
            StreamAddress::address(root.path(), extension, &source),
            StreamAddress::QUERIED_ADDRESS,
            "{extension}"
        );
    }
}

#[test]
fn a_stream_query_needs_a_model_name_and_fills_well_formed_path_parts() {
    let root = StreamAddress::project();
    let template = StreamAddress::CLIENTS[1].1;
    for (arguments, said) in [
        (r#""/stats/{window}""#, "needs a query model"),
        (r#"{"path": "/stats/{window}"}"#, "needs a query model"),
        (
            r#"{"path": "/stats/{window", "query": "StatsWindow"}"#,
            "{name} part",
        ),
        (
            r#"{"path": "/stats/window}", "query": "StatsWindow"}"#,
            "{name} part",
        ),
        (
            r#"{"path": "/stats/{1window}", "query": "StatsWindow"}"#,
            "{name} part",
        ),
        (
            r#"{"path": "/stats/{win-dow}", "query": "StatsWindow"}"#,
            "{name} part",
        ),
        (
            r#"{"path": "/stats", "query": "Stats.Window"}"#,
            "names the model",
        ),
        (r#"{"path": "/stats", "query": ""}"#, "names the model"),
        (r#"{"path": "/stats", "query": 3}"#, "names the model"),
        (r#""/.tachyon/query/StatsWindow/stats""#, "/.tachyon/query/"),
    ] {
        let source = StreamAddress::stream_template(template, arguments);
        let refused = StreamAddress::extracted(root.path(), "ts", &source, "\"/chat\"")
            .expect_err(arguments)
            .to_string();
        assert!(refused.contains(said), "{arguments}: {refused}");
    }
}

#[test]
fn a_stream_query_without_path_parts_keeps_the_plain_path() {
    let root = StreamAddress::project();
    let template = StreamAddress::CLIENTS[1].1;
    let source =
        StreamAddress::stream_template(template, r#"{"path": "/stats", "query": "StatsWindow"}"#);
    let module = StreamAddress::extracted(root.path(), "ts", &source, "\"/chat\"")
        .expect("queried")
        .expect("module");
    assert_eq!(
        module.types[0].streams[0].path,
        "/.tachyon/query/StatsWindow/chat/stats"
    );
}

#[test]
fn a_socket_request_refuses_a_query() {
    let root = StreamAddress::project();
    for (extension, template) in StreamAddress::CLIENTS {
        let source = template.replace("SOCKET", r#"{"path": "/ws", "query": "Room"}"#);
        let refused = StreamAddress::extracted(root.path(), extension, &source, "\"/chat\"")
            .expect_err(extension)
            .to_string();
        assert!(
            refused.contains("@SocketRequest takes no query"),
            "{extension}: {refused}"
        );
    }
}
