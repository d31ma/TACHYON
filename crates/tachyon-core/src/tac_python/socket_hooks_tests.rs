use super::page_tests::PythonPage;
use crate::tac_web::WebLanguage;
use crate::tac_web::socket_hooks_shared_tests::{SocketHooks, SocketHooksCase};

struct PythonHooks;

impl PythonHooks {
    const MANIFEST: &'static str =
        "{\"tac\":{\"python\":{\"runtime\":\"pyodide-314.0.7\",\"profile\":\"offline\"}}}\n";

    const FILES: [(&'static str, &'static str); 2] = [
        (
            "client/models/ChatMessage.py",
            "from dataclasses import dataclass\n\n@dataclass\nclass ChatMessage:\n    text: str\n",
        ),
        (
            "client/remotes/ChatClient.py",
            "from typing import Protocol\nfrom models import ChatMessage\n\n@http_client(\"/chat\")\nclass ChatClient(Protocol):\n    @socket_request\n    def receive(self, message: ChatMessage) -> None: ...\n",
        ),
    ];

    const SERVICE: &'static str = "from typing import Annotated\n\n@service\nclass ChatService:\n    chat: Annotated[ChatClient, needs]\n    opens: int = 0\n    reopens: int = 0\n    code: float = 0\n    retrying: bool = False\n    ticks: int = 0\n\n    \
         @on_event\n    def connected(self, opened: SocketOpened) -> None:\n        self.opens += 1\n        self.reopens += 1 if opened.reconnected else 0\n\n    \
         @on_event\n    def disconnected(self, closed: SocketClosed) -> None:\n        self.code = closed.code\n        self.retrying = closed.reconnecting\n\n    \
         @every(\"30s\")\n    async def refresh(self) -> None:\n        self.ticks += 1\n";
}

#[tokio::test]
async fn a_python_service_hears_its_socket_and_runs_its_every_method() {
    if let Some(reason) = PythonPage::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let case = SocketHooksCase {
        language: WebLanguage::Python,
        extension: "py",
        files: &PythonHooks::FILES,
        service: PythonHooks::SERVICE,
        sockets: true,
    };
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    SocketHooks::write(
        root.path(),
        "client/manifest.json",
        PythonHooks::MANIFEST.as_bytes(),
    );
    SocketHooks::prepare(root.path(), &case).await;
    let report = PythonPage::run(
        root.path(),
        &SocketHooks::body(true, "application-events.mjs"),
    );
    SocketHooks::check(&report, true);
}
