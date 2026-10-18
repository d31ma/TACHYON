use super::{CppYon, CppYonController};
use crate::yon::plan::{ParameterSource, SocketMethod};
use crate::yon::route_access::RouteAccess;
use std::path::Path;

struct Samples;

impl Samples {
    const TICKS_PATH: &'static str = "server/controllers/TicksController.hpp";
    const TICKS: &'static str = r#"#pragma once
#include <optional>

class [[RequestRoute("/ticks")]] [[Public]] TicksController {
public:
    [[StreamRoute]]
    tachyon::Generator<Tick> streamTicks([[FromQuery]] std::optional<int> count) {
        co_yield Tick{};
    }

    [[GetRoute("/last")]]
    Tick getLast() {
        return Tick{};
    }
};
"#;
    const CHAT_PATH: &'static str = "server/controllers/ChatController.hpp";
    const CHAT: &'static str = r#"#pragma once
#include <string>

class [[SocketRoute("/chat")]] [[SignedIn]] ChatController {
public:
    [[OnOpen]]
    void onOpen(SocketSession& session) {}

    [[OnMessage]]
    void onMessage(SocketSession& session, const ChatMessage& message) {}

    [[OnError]]
    void onError(SocketSession& session, const std::string& error) {}

private:
    int count = 0;
};
"#;

    fn read(path: &str, source: &str) -> Result<CppYonController, crate::Failure> {
        CppYonController::read(Path::new(path), source)
    }

    fn refusal(path: &str, sample: &str, from: &str, to: &str) -> String {
        let source = sample.replace(from, to);
        assert_ne!(source, sample, "the sample changes");
        Self::read(path, &source)
            .err()
            .expect("the controller is refused")
            .to_string()
    }
}

#[test]
fn a_stream_route_is_a_get_route_that_streams() {
    let controller = Samples::read(Samples::TICKS_PATH, Samples::TICKS).expect("controller");
    let stream = &controller.routes[0];
    assert_eq!(stream.verb, "GET");
    assert_eq!(stream.method, "streamTicks");
    assert_eq!(stream.pattern.as_deref(), Some("/ticks"));
    assert!(stream.streaming);
    assert_eq!(stream.parameters[0].source, ParameterSource::Query);
    assert!(!controller.routes[1].streaming);
    let streaming =
        CppYon::streaming(Path::new(Samples::TICKS_PATH), Samples::TICKS).expect("a C++ file");
    assert_eq!(streaming.into_iter().collect::<Vec<_>>(), ["GET"]);
}

#[test]
fn a_stream_route_names_its_shape_when_it_is_wrong() {
    let cases = [
        (
            "tachyon::Generator<Tick> streamTicks(",
            "std::vector<Tick> streamTicks(",
            "returns tachyon::Generator<Model>",
        ),
        (
            "tachyon::Generator<Tick> streamTicks(",
            "tachyon::Generator<Tick> ticks(",
            "starts with 'stream'",
        ),
        (
            "Tick getLast()",
            "tachyon::Generator<Tick> getLast()",
            "mark it [[StreamRoute]]",
        ),
    ];
    for (from, to, expected) in cases {
        let failure = Samples::refusal(Samples::TICKS_PATH, Samples::TICKS, from, to);
        assert!(failure.contains("TY2015"), "{to}: {failure}");
        assert!(failure.contains(expected), "{to}: {failure}");
    }
}

#[test]
fn a_socket_route_lists_its_event_methods_and_message_model() {
    let controller = Samples::read(Samples::CHAT_PATH, Samples::CHAT).expect("controller");
    let [route] = controller.routes.as_slice() else {
        panic!("one socket route: {:?}", controller.routes);
    };
    assert_eq!(route.verb, "GET");
    assert_eq!(route.pattern.as_deref(), Some("/chat"));
    assert_eq!(route.access, RouteAccess::SignedIn);
    assert_eq!(route.parameters[0].source, ParameterSource::Body);
    assert_eq!(route.parameters[0].type_text, "ChatMessage");
    let methods = route.socket.as_ref().expect("socket methods");
    assert_eq!(methods.message, "onMessage");
    assert_eq!(
        methods.open,
        Some(SocketMethod {
            name: String::from("onOpen"),
            parameters: 1
        })
    );
    assert_eq!(methods.close, None);
    assert_eq!(
        methods.error.as_ref().map(|method| method.parameters),
        Some(2)
    );
}

#[test]
fn a_socket_route_refuses_shapes_it_cannot_run() {
    let cases = [
        (
            "[[SocketRoute(\"/chat\")]]",
            "[[SocketRoute(\"/chat/{room}\")]]",
            "has no {variables}",
        ),
        (
            "    [[OnMessage]]\n    void onMessage(SocketSession& session, const ChatMessage& message) {}\n",
            "",
            "needs one [[OnMessage]] method",
        ),
        (
            "    void onMessage(SocketSession& session, const ChatMessage& message) {}",
            "    void onMessage(SocketSession& session) {}",
            "takes (SocketSession& session, ChatMessage message)",
        ),
        (
            "    void onOpen(SocketSession& session) {}",
            "    bool onOpen(SocketSession& session) { return true; }",
            "returns void",
        ),
        (
            "private:\n",
            "    [[GetRoute(\"/rooms\")]]\n    int getRooms() { return 1; }\n\nprivate:\n",
            "belongs in a class marked [[RequestRoute]]",
        ),
        (
            "[[OnError]]\n    void onError(",
            "[[OnOpen]]\n    void onError(",
            "at most one [[OnOpen]] method",
        ),
    ];
    for (from, to, expected) in cases {
        let failure = Samples::refusal(Samples::CHAT_PATH, Samples::CHAT, from, to);
        assert!(failure.contains("TY2015"), "{to}: {failure}");
        assert!(failure.contains(expected), "{to}: {failure}");
    }
}

#[test]
fn the_programs_stream_items_and_run_socket_events() {
    let root = tempfile::tempdir().expect("root");
    let ticks =
        CppYon::lower(Path::new(Samples::TICKS_PATH), Samples::TICKS, root.path()).expect("ticks");
    assert!(ticks.source.contains(
        "return tachyon::yon::YonStream::send(frame, invocation, [&]() { return controller.streamTicks(frame.argument<std::optional<int>>(0)); });"
    ));
    let chat =
        CppYon::lower(Path::new(Samples::CHAT_PATH), Samples::CHAT, root.path()).expect("chat");
    assert!(
        chat.source
            .contains("tachyon::yon::SocketSession session(frame.text(\"request_id\"));")
    );
    assert!(chat.source.contains(
        "controller.onMessage(session, tachyon::yon::YonSocket::message<ChatMessage>(value));"
    ));
    assert!(
        chat.source
            .contains("[&]() { controller.onOpen(session); }")
    );
    assert!(chat.source.contains("controller.onError(session, error);"));
    assert!(chat.routes[0].socket.is_some());
}
