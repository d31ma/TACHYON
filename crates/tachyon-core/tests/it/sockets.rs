#![allow(clippy::expect_used)]

use futures_util::{SinkExt as _, StreamExt as _};
use std::fs;
use std::path::Path;
use std::time::Duration;
use tachyon_core::{DevServer, DevServerOptions};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest as _;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

struct ChatProject {
    program: &'static str,
    files: &'static [(&'static str, &'static str)],
}

impl ChatProject {
    const TYPESCRIPT: Self = Self {
        program: "node",
        files: &[
            (
                "server/models/ChatMessage.ts",
                "export interface ChatMessage {\n  text: string\n}\n",
            ),
            (
                "server/controllers/ChatController.ts",
                r"import type { ChatMessage } from '/models/ChatMessage.ts'

@SocketRoute('/chat')
@Public
export class ChatController {
  static readonly ALL = 'all'
  count = 0

  @OnOpen
  onOpen(session: any) {
    return session.send({ text: 'welcome' })
  }

  @OnMessage
  async onMessage(session: any, message: ChatMessage) {
    this.count += 1
    await this.sendHello(await this.peersFor(session, message))
    await this.answer(session, message)
  }

  private async peersFor(session: any, message: ChatMessage) {
    if (message.text !== ChatController.ALL) return []
    return session.openSessions()
  }

  private async sendHello(peers: any[]) {
    for (const peer of peers) await peer.send({ text: 'hello all' })
  }

  private async answer(session: any, message: ChatMessage) {
    switch (message.text) {
      case 'fail': throw new Error('secret-canary')
      case ChatController.ALL: return
    }
    await session.send({ text: `${message.text} ${this.count}` })
  }

  @OnError
  onError(session: any) {
    return session.send({ text: 'sorry' })
  }
}
",
            ),
        ],
    };

    const PYTHON: Self = Self {
        program: "python3",
        files: &[
            (
                "server/models/ChatMessage.py",
                "from dataclasses import dataclass\n\n\n@dataclass\nclass ChatMessage:\n    text: str\n",
            ),
            (
                "server/controllers/ChatController.py",
                r#"from models import ChatMessage


@socket_route("/chat")
@public
class ChatController:
    ALL = "all"
    def __init__(self):
        self.count = 0

    @on_open
    def on_open(self, session):
        session.send(ChatMessage("welcome"))

    @on_message
    def on_message(self, session, message: ChatMessage):
        self.count += 1
        self._send_hello(self._peers_for(session, message))
        self._answer(session, message)

    def _peers_for(self, session, message):
        if message.text != self.ALL:
            return []
        return session.open_sessions()

    def _send_hello(self, peers):
        for peer in peers:
            peer.send(ChatMessage("hello all"))

    def _answer(self, session, message):
        match message.text:
            case "fail":
                raise RuntimeError("secret-canary")
            case self.ALL:
                return
        session.send(ChatMessage(f"{message.text} {self.count}"))

    @on_error
    def on_error(self, session, error):
        session.send(ChatMessage("sorry"))
"#,
            ),
        ],
    };

    const GO: Self = Self {
        program: "go",
        files: &[
            (
                "server/models/ChatMessage.go",
                "type ChatMessage struct {\n\tText string `json:\"text\"`\n}\n",
            ),
            (
                "server/controllers/ChatController.go",
                r#"import (
	"errors"
	"fmt"
)

// @SocketRoute("/chat")
// @Public
type ChatController struct {
	count int
}

// @OnOpen
func (controller *ChatController) OnOpen(session *SocketSession) error {
	return session.Send(ChatMessage{Text: "welcome"})
}

// @OnMessage
func (controller *ChatController) OnMessage(session *SocketSession, message ChatMessage) error {
	controller.count++
	controller.sendHello(controller.peersFor(session, message))
	return controller.answer(session, message)
}

func (controller *ChatController) peersFor(session *SocketSession, message ChatMessage) []SocketPeer {
	if message.Text != controller.all() {
		return nil
	}
	return session.OpenSessions()
}

func (controller *ChatController) sendHello(peers []SocketPeer) {
	for _, peer := range peers {
		peer.Send(ChatMessage{Text: "hello all"})
	}
}

func (controller *ChatController) answer(session *SocketSession, message ChatMessage) error {
	switch message.Text {
	case "fail":
		return errors.New("secret-canary")
	case controller.all():
		return nil
	}
	return session.Send(ChatMessage{Text: fmt.Sprintf("%s %d", message.Text, controller.count)})
}

func (ChatController) all() string { return "all" }

// @OnError
func (controller *ChatController) OnError(session *SocketSession, failure error) {
	session.Send(ChatMessage{Text: "sorry"})
}
"#,
            ),
        ],
    };

    const JAVA: Self = Self {
        program: "javac",
        files: &[
            (
                "server/models/ChatMessage.java",
                "record ChatMessage(String text) {}\n",
            ),
            (
                "server/controllers/ChatController.java",
                r#"import models.ChatMessage;

@SocketRoute("/chat")
@Public
class ChatController {
    private static final String ALL = "all";
    private int count;

    @OnOpen
    void onOpen(SocketSession session) throws Exception {
        session.send(new ChatMessage("welcome"));
    }

    @OnMessage
    void onMessage(SocketSession session, ChatMessage message) throws Exception {
        count += 1;
        sendHello(peersFor(session, message));
        answer(session, message);
    }

    private java.util.List<SocketPeer> peersFor(SocketSession session, ChatMessage message) throws Exception {
        if (!message.text().equals(ALL)) return java.util.List.of();
        return session.getOpenSessions();
    }

    private void sendHello(java.util.List<SocketPeer> peers) throws Exception {
        for (SocketPeer peer : peers) peer.send(new ChatMessage("hello all"));
    }

    private void answer(SocketSession session, ChatMessage message) throws Exception {
        switch (message.text()) {
            case "fail" -> throw new IllegalStateException("secret-canary");
            case ALL -> { }
            default -> session.send(new ChatMessage(message.text() + " " + count));
        }
    }

    @OnError
    void onError(SocketSession session, Throwable error) throws Exception {
        session.send(new ChatMessage("sorry"));
    }
}
"#,
            ),
        ],
    };

    const KOTLIN: Self = Self {
        program: "kotlinc",
        files: &[
            (
                "server/models/ChatMessage.kt",
                "data class ChatMessage(val text: String)\n",
            ),
            (
                "server/controllers/ChatController.kt",
                r#"import models.ChatMessage

@SocketRoute("/chat")
@Public
class ChatController {
    private var count = 0

    companion object {
        private const val ALL = "all"
    }

    @OnOpen
    fun onOpen(session: SocketSession) {
        session.send(ChatMessage("welcome"))
    }

    @OnMessage
    fun onMessage(session: SocketSession, message: ChatMessage) {
        count += 1
        sendHello(peersFor(session, message))
        answer(session, message)
    }

    private fun peersFor(session: SocketSession, message: ChatMessage): List<SocketPeer> {
        if (message.text != ALL) return emptyList()
        return session.openSessions()
    }

    private fun sendHello(peers: List<SocketPeer>) {
        for (peer in peers) peer.send(ChatMessage("hello all"))
    }

    private fun answer(session: SocketSession, message: ChatMessage) {
        when (message.text) {
            "fail" -> throw IllegalStateException("secret-canary")
            ALL -> Unit
            else -> session.send(ChatMessage("${message.text} $count"))
        }
    }

    @OnError
    fun onError(session: SocketSession, error: Throwable) {
        session.send(ChatMessage("sorry"))
    }
}
"#,
            ),
        ],
    };

    const CSHARP: Self = Self {
        program: "dotnet",
        files: &[
            (
                "server/models/ChatMessage.cs",
                "public record ChatMessage(string Text);\n",
            ),
            (
                "server/controllers/ChatController.cs",
                r#"using System;
using Models;

[SocketRoute("/chat")]
[Public]
public class ChatController
{
    private const string All = "all";
    private int count;

    [OnOpen]
    public void OnOpen(SocketSession session)
    {
        session.Send(new ChatMessage("welcome"));
    }

    [OnMessage]
    public void OnMessage(SocketSession session, ChatMessage message)
    {
        count += 1;
        SendHello(PeersFor(session, message));
        Answer(session, message);
    }

    private static List<SocketPeer> PeersFor(SocketSession session, ChatMessage message)
    {
        if (message.Text != All) return new List<SocketPeer>();
        return session.OpenSessions();
    }

    private static void SendHello(List<SocketPeer> peers)
    {
        foreach (var peer in peers) peer.Send(new ChatMessage("hello all"));
    }

    private void Answer(SocketSession session, ChatMessage message)
    {
        switch (message.Text)
        {
            case "fail": throw new InvalidOperationException("secret-canary");
            case All: return;
        }
        session.Send(new ChatMessage($"{message.Text} {count}"));
    }

    [OnError]
    public void OnError(SocketSession session, Exception error)
    {
        session.Send(new ChatMessage("sorry"));
    }
}
"#,
            ),
        ],
    };

    const PHP: Self = Self {
        program: "php-cgi",
        files: &[
            (
                "server/models/ChatMessage.php",
                "<?php\nclass ChatMessage {\n    public function __construct(public string $text) {}\n}\n",
            ),
            (
                "server/controllers/ChatController.php",
                r#"<?php
use Models\ChatMessage;

#[SocketRoute("/chat")]
#[Public]
class ChatController {
    private const ALL = 'all';
    private int $count = 0;

    #[OnOpen]
    public function onOpen(SocketSession $session): void {
        $session->send(new ChatMessage('welcome'));
    }

    #[OnMessage]
    public function onMessage(SocketSession $session, ChatMessage $message): void {
        $this->count += 1;
        $this->sendHello($this->peersFor($session, $message));
        $this->answer($session, $message);
    }

    private function peersFor(SocketSession $session, ChatMessage $message): array {
        if ($message->text !== self::ALL) {
            return [];
        }
        return $session->openSessions();
    }

    private function sendHello(array $peers): void {
        foreach ($peers as $peer) {
            $peer->send(new ChatMessage('hello all'));
        }
    }

    private function answer(SocketSession $session, ChatMessage $message): void {
        switch ($message->text) {
            case 'fail':
                throw new RuntimeException('secret-canary');
            case self::ALL:
                return;
        }
        $session->send(new ChatMessage($message->text . ' ' . $this->count));
    }

    #[OnError]
    public function onError(SocketSession $session, Throwable $error): void {
        $session->send(new ChatMessage('sorry'));
    }
}
"#,
            ),
        ],
    };

    const RUBY: Self = Self {
        program: "ruby",
        files: &[
            (
                "server/models/ChatMessage.rb",
                "# @!attribute [r] text\n#   @return [String]\nChatMessage = Data.define(:text)\n",
            ),
            (
                "server/controllers/ChatController.rb",
                r##"#[SocketRoute("/chat")]
#[Public]
class ChatController
  ALL = "all"

  def initialize
    @count = 0
  end

  #[OnOpen]
  def on_open(session)
    session.send(ChatMessage.new(text: "welcome"))
  end

  # @param message [ChatMessage]
  #[OnMessage]
  def on_message(session, message)
    @count += 1
    send_hello(peers_for(session, message))
    answer(session, message)
  end

  #[OnError]
  def on_error(session, error)
    session.send(ChatMessage.new(text: "sorry"))
  end

  private

  def peers_for(session, message)
    return [] unless message.text == ALL

    session.open_sessions
  end

  def send_hello(peers)
    peers.each { |peer| peer.send(ChatMessage.new(text: "hello all")) }
  end

  def answer(session, message)
    case message.text
    when "fail" then raise "secret-canary"
    when ALL then nil
    else session.send(ChatMessage.new(text: "#{message.text} #{@count}"))
    end
  end
end
"##,
            ),
        ],
    };

    const RUST: Self = Self {
        program: "rustc",
        files: &[
            (
                "server/models/ChatMessage.rs",
                "pub struct ChatMessage {\n    pub text: String,\n}\n",
            ),
            (
                "server/controllers/ChatController.rs",
                r#"use crate::models::ChatMessage;

#[SocketRoute("/chat")]
#[Public]
struct ChatController {
    count: std::cell::Cell<i64>,
}

impl ChatController {
    const ALL: &str = "all";

    fn new() -> Self {
        Self { count: std::cell::Cell::new(0) }
    }

    #[OnOpen]
    fn on_open(&self, session: &SocketSession) {
        session.send(ChatMessage { text: String::from("welcome") });
    }

    #[OnMessage]
    fn on_message(&self, session: &SocketSession, message: ChatMessage) -> Result<(), String> {
        self.count.set(self.count.get() + 1);
        Self::send_hello(Self::peers_for(session, &message));
        Self::answer(session, &message, self.count.get())
    }

    fn peers_for(session: &SocketSession, message: &ChatMessage) -> Vec<SocketPeer> {
        if message.text != Self::ALL {
            return Vec::new();
        }
        session.open_sessions()
    }

    fn send_hello(peers: Vec<SocketPeer>) {
        for peer in peers {
            peer.send(ChatMessage { text: String::from("hello all") });
        }
    }

    fn answer(session: &SocketSession, message: &ChatMessage, count: i64) -> Result<(), String> {
        match message.text.as_str() {
            "fail" => Err(String::from("secret-canary")),
            Self::ALL => Ok(()),
            _ => {
                session.send(ChatMessage { text: format!("{} {}", message.text, count) });
                Ok(())
            }
        }
    }

    #[OnError]
    fn on_error(&self, session: &SocketSession, _error: &str) {
        session.send(ChatMessage { text: String::from("sorry") });
    }
}
"#,
            ),
        ],
    };

    const DART: Self = Self {
        program: "dart",
        files: &[
            (
                "server/models/ChatMessage.dart",
                "class ChatMessage {\n  final String text;\n\n  const ChatMessage(this.text);\n}\n",
            ),
            (
                "server/controllers/ChatController.dart",
                r#"import 'models/ChatMessage.dart';

@SocketRoute("/chat")
@Public
class ChatController {
  static const all = 'all';
  int count = 0;

  @OnOpen
  Future<void> onOpen(SocketSession session) async {
    await session.send(const ChatMessage('welcome'));
  }

  @OnMessage
  Future<void> onMessage(SocketSession session, ChatMessage message) async {
    count += 1;
    await _sendHello(await _peersFor(session, message));
    await _answer(session, message);
  }

  Future<List<SocketPeer>> _peersFor(SocketSession session, ChatMessage message) async {
    if (message.text != all) return [];
    return session.openSessions();
  }

  Future<void> _sendHello(List<SocketPeer> peers) async {
    for (final peer in peers) {
      await peer.send(const ChatMessage('hello all'));
    }
  }

  Future<void> _answer(SocketSession session, ChatMessage message) async {
    switch (message.text) {
      case 'fail':
        throw StateError('secret-canary');
      case all:
        return;
    }
    await session.send(ChatMessage('${message.text} $count'));
  }

  @OnError
  void onError(SocketSession session, Object error) {
    session.send(const ChatMessage('sorry'));
  }
}
"#,
            ),
        ],
    };

    fn write(root: &Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        fs::write(path, contents).expect("source");
    }

    fn create(&self, root: &Path) {
        Self::write(root, "client/pages/tac.html", "<test-view1 />");
        Self::write(
            root,
            "client/components/test/view1/tac.html",
            "<main aria-label=\"Chat\"><h1>Chat</h1></main>",
        );
        for (relative, contents) in self.files {
            Self::write(root, relative, contents);
        }
    }

    fn available(&self) -> bool {
        let program = if self.program == "node" {
            std::env::var("YON_JAVASCRIPT_RUNTIME").unwrap_or_else(|_| String::from("node"))
        } else {
            self.program.to_owned()
        };
        let flag = match program.as_str() {
            "go" => "version",
            "kotlinc" => "-version",
            _ => "--version",
        };
        let found = std::process::Command::new(&program)
            .arg(flag)
            .output()
            .is_ok_and(|output| output.status.success());
        assert!(
            found || std::env::var_os("CI").is_none(),
            "{program} is missing in CI"
        );
        if !found {
            eprintln!("skipping: {program} is unavailable");
        }
        found
    }
}

struct Server {
    port: u16,
    stop: tokio::sync::oneshot::Sender<()>,
    running: tokio::task::JoinHandle<()>,
    _project: tempfile::TempDir,
}

impl Server {
    async fn start(chat: &ChatProject) -> Self {
        let project = tempfile::tempdir().expect("project");
        chat.create(project.path());
        let server = DevServer::bind(
            project.path(),
            &DevServerOptions {
                port: 0,
                watch: false,
                ..DevServerOptions::default()
            },
        )
        .await
        .expect("bind");
        let port = server.address().port();
        let (stop, shutdown) = tokio::sync::oneshot::channel();
        let running = tokio::spawn(async move {
            let _served = server
                .run_until(async {
                    let _stopped = shutdown.await;
                })
                .await;
        });
        Self {
            port,
            stop,
            running,
            _project: project,
        }
    }

    async fn stop(self) {
        let _stopped = self.stop.send(());
        let _finished = self.running.await;
    }

    async fn chat(&self) {
        let port = self.port;
        let mut first = Client::open(port).await;
        assert_eq!(Client::text(&mut first).await, r#"{"text":"welcome"}"#);
        Client::send(&mut first, r#"{"text":"hi"}"#).await;
        assert_eq!(Client::text(&mut first).await, r#"{"text":"hi 1"}"#);
        Client::send(&mut first, r#"{"text":"again"}"#).await;
        assert_eq!(Client::text(&mut first).await, r#"{"text":"again 2"}"#);

        let mut second = Client::open(port).await;
        assert_eq!(Client::text(&mut second).await, r#"{"text":"welcome"}"#);
        Client::send(&mut second, r#"{"text":"hi"}"#).await;
        assert_eq!(Client::text(&mut second).await, r#"{"text":"hi 1"}"#);

        Client::send(&mut first, r#"{"text":"all"}"#).await;
        assert_eq!(Client::text(&mut first).await, r#"{"text":"hello all"}"#);
        assert_eq!(Client::text(&mut second).await, r#"{"text":"hello all"}"#);

        Client::send(&mut first, r#"{"words":1}"#).await;
        let (code, reason) = Client::close_code(&mut first).await;
        assert_eq!(code, 1007, "{reason}");
        assert!(reason.contains("does not fit"), "{reason}");

        Client::send(&mut second, r#"{"text":"fail"}"#).await;
        assert_eq!(Client::text(&mut second).await, r#"{"text":"sorry"}"#);
        Client::send(&mut second, r#"{"text":"after"}"#).await;
        let after = Client::text(&mut second).await;
        assert_eq!(after, r#"{"text":"after 3"}"#);
        assert!(!after.contains("secret-canary"), "{after}");
    }
}

struct Client;

impl Client {
    async fn open(port: u16) -> Socket {
        let url = format!("ws://127.0.0.1:{port}/chat");
        let (socket, _response) = tokio_tungstenite::connect_async(url)
            .await
            .expect("socket opens");
        socket
    }

    async fn send(socket: &mut Socket, text: &str) {
        socket
            .send(Message::Text(text.into()))
            .await
            .expect("message sent");
    }

    async fn next(socket: &mut Socket) -> Message {
        loop {
            let message = tokio::time::timeout(Duration::from_secs(30), socket.next())
                .await
                .expect("message within the deadline")
                .expect("socket still open")
                .expect("readable message");
            if !matches!(message, Message::Ping(_) | Message::Pong(_)) {
                return message;
            }
        }
    }

    async fn text(socket: &mut Socket) -> String {
        match Self::next(socket).await {
            Message::Text(text) => text.to_string(),
            other => panic!("expected text, got {other:?}"),
        }
    }

    async fn close_code(socket: &mut Socket) -> (u16, String) {
        match Self::next(socket).await {
            Message::Close(Some(frame)) => (u16::from(frame.code), frame.reason.to_string()),
            other => panic!("expected a close frame, got {other:?}"),
        }
    }

    async fn raw(port: u16, request: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port))
            .await
            .expect("connect");
        stream.write_all(request.as_bytes()).await.expect("write");
        let mut response = Vec::new();
        tokio::time::timeout(Duration::from_secs(30), stream.read_to_end(&mut response))
            .await
            .expect("response within the deadline")
            .expect("read");
        String::from_utf8_lossy(&response).into_owned()
    }
}

#[tokio::test]
async fn a_socket_route_keeps_one_object_for_each_connection_and_guards_it() {
    if !ChatProject::TYPESCRIPT.available() {
        return;
    }
    let server = Server::start(&ChatProject::TYPESCRIPT).await;
    server.chat().await;
    let port = server.port;

    let mut third = Client::open(port).await;
    assert_eq!(Client::text(&mut third).await, r#"{"text":"welcome"}"#);
    third
        .send(Message::Binary(vec![1_u8, 2].into()))
        .await
        .expect("binary sent");
    let (code, _reason) = Client::close_code(&mut third).await;
    assert_eq!(code, 1003);

    let plain = Client::raw(
        port,
        "GET /chat HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    )
    .await;
    assert!(plain.starts_with("HTTP/1.1 426"), "{plain}");

    let mut foreign = format!("ws://127.0.0.1:{port}/chat")
        .into_client_request()
        .expect("request");
    foreign.headers_mut().insert(
        "origin",
        "https://elsewhere.example".parse().expect("origin"),
    );
    let refused = tokio_tungstenite::connect_async(foreign).await;
    let Err(tokio_tungstenite::tungstenite::Error::Http(response)) = refused else {
        panic!("a foreign origin must be refused");
    };
    assert_eq!(response.status().as_u16(), 403);

    let mut same = format!("ws://127.0.0.1:{port}/chat")
        .into_client_request()
        .expect("request");
    same.headers_mut().insert(
        "origin",
        format!("http://127.0.0.1:{port}").parse().expect("origin"),
    );
    let (mut same, _response) = tokio_tungstenite::connect_async(same)
        .await
        .expect("same origin opens");
    assert_eq!(Client::text(&mut same).await, r#"{"text":"welcome"}"#);

    server.stop().await;
}

#[tokio::test]
async fn a_python_socket_route_keeps_one_object_for_each_connection() {
    if !ChatProject::PYTHON.available() {
        return;
    }
    let server = Server::start(&ChatProject::PYTHON).await;
    server.chat().await;
    server.stop().await;
}

#[tokio::test]
async fn a_go_socket_route_keeps_one_object_for_each_connection() {
    if !ChatProject::GO.available() {
        return;
    }
    let server = Server::start(&ChatProject::GO).await;
    server.chat().await;
    server.stop().await;
}

#[tokio::test]
async fn a_java_socket_route_keeps_one_object_for_each_connection() {
    if !ChatProject::JAVA.available() {
        return;
    }
    let server = Server::start(&ChatProject::JAVA).await;
    server.chat().await;
    server.stop().await;
}

#[tokio::test]
async fn a_kotlin_socket_route_keeps_one_object_for_each_connection() {
    if !ChatProject::KOTLIN.available() {
        return;
    }
    let server = Server::start(&ChatProject::KOTLIN).await;
    server.chat().await;
    server.stop().await;
}

#[tokio::test]
async fn a_csharp_socket_route_keeps_one_object_for_each_connection() {
    if !ChatProject::CSHARP.available() {
        return;
    }
    let server = Server::start(&ChatProject::CSHARP).await;
    server.chat().await;
    server.stop().await;
}

#[tokio::test]
async fn a_php_socket_route_keeps_one_object_for_each_connection() {
    if !ChatProject::PHP.available() {
        return;
    }
    let server = Server::start(&ChatProject::PHP).await;
    server.chat().await;
    server.stop().await;
}

#[tokio::test]
async fn a_ruby_socket_route_keeps_one_object_for_each_connection() {
    if !ChatProject::RUBY.available() {
        return;
    }
    let server = Server::start(&ChatProject::RUBY).await;
    server.chat().await;
    server.stop().await;
}

#[tokio::test]
async fn a_rust_socket_route_keeps_one_object_for_each_connection() {
    if !ChatProject::RUST.available() {
        return;
    }
    let server = Server::start(&ChatProject::RUST).await;
    server.chat().await;
    server.stop().await;
}

#[tokio::test]
async fn a_dart_socket_route_keeps_one_object_for_each_connection() {
    if !ChatProject::DART.available() {
        return;
    }
    let server = Server::start(&ChatProject::DART).await;
    server.chat().await;
    server.stop().await;
}
