use super::page_tests::PythonPage;
use crate::tac_web::{BuildJobs, WebLanguage};

struct CheckedPage;

impl CheckedPage {
    const MANIFEST: &'static str =
        "{\"tac\":{\"python\":{\"runtime\":\"pyodide-314.0.7\",\"profile\":\"offline\"}}}\n";
    const MODEL: &'static str = "from typing import Annotated\n\n@checked\nclass OrderInput:\n    name: Annotated[str, not_empty, length({\"min\":1,\"max\":80})] = ''\n    level: Annotated[int, required, min(1), max(9)] = 0\n";
    const SERVICE: &'static str = "from typing import Annotated\nfrom models import OrderInput\n\n@service\nclass OrderService:\n    status: str = 'ready'\n\n    def accept(self, order: Annotated[OrderInput, check]) -> OrderInput:\n        return order\n";
}

#[tokio::test]
async fn a_python_check_of_a_client_model_stops_the_build() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    PythonPage::write(root.path(), "client/manifest.json", CheckedPage::MANIFEST);
    PythonPage::write(
        root.path(),
        "client/models/OrderInput.py",
        CheckedPage::MODEL,
    );
    let service = PythonPage::write(
        root.path(),
        "client/services/OrderService.py",
        CheckedPage::SERVICE,
    );
    let chosen = [(
        String::from("order"),
        String::from("OrderService"),
        WebLanguage::Python,
        service,
    )];
    let built = crate::tac_web::build_all(
        root.path(),
        &chosen,
        &BuildJobs::new(1),
        crate::BuildProfile::Release,
    )
    .await;
    let Err(failure) = built else {
        panic!("the build accepted a Python Check of a client model");
    };
    let diagnostic = &failure.diagnostics()[0];
    assert_eq!(diagnostic.code.number(), 1308, "{failure}");
    assert!(
        diagnostic
            .message
            .contains("Check requires a source-local Checked type"),
        "{failure}"
    );
    assert_eq!(
        diagnostic.spans[0].file, "client/services/OrderService.py",
        "{failure}"
    );
}
