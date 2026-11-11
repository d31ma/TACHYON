use super::page_tests::PythonPage;
use crate::tac_web::WebLanguage;
use crate::tac_web::socket_hooks_harness::{SocketHooks, SocketHooksCase};

struct PythonEvery;

impl PythonEvery {
    const SERVICE: &'static str = "@service\nclass ChatService:\n    ticks: int = 0\n\n    \
         @every(\"30s\")\n    def refresh(self) -> None:\n        self.ticks += 1\n";
}

#[tokio::test]
async fn a_python_service_runs_its_every_method_while_the_page_is_shown() {
    if let Some(reason) = PythonPage::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let case = SocketHooksCase {
        language: WebLanguage::Python,
        extension: "py",
        files: &[],
        service: PythonEvery::SERVICE,
        sockets: false,
    };
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    let report = SocketHooks::run(root.path(), &case).await;
    SocketHooks::check(&report, false);
}
