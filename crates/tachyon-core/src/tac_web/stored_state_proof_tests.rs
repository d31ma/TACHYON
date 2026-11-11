use crate::one_test_per_case_tests::one_test_per_case;
use serde_json::json;

struct StoredStateProof;

impl StoredStateProof {
    const LANGUAGES: [&'static str; 3] = ["dart", "go", "rs"];

    async fn run(language: &str) -> (serde_json::Value, Option<serde_json::Value>) {
        match language {
            "dart" => {
                crate::tac_dart::dart_marker_proof_tests::DartMarkerProofs::stored_state().await
            }
            "go" => super::go_marker_proof_tests::GoMarkerProofs::stored_state().await,
            "rs" => super::rust_marker_proof_tests::RustMarkerProofs::stored_state().await,
            other => panic!("no stored-state proof uses .{other}"),
        }
    }
}

async fn a_service_writes_its_local_state_to_page_storage_and_restores_it_in(language: &str) {
    let (stored, report) = StoredStateProof::run(language).await;
    assert_eq!(stored, json!({"draft": "session", "visits": "local"}));
    let Some(report) = report else {
        return;
    };
    assert_eq!(
        report["refused"],
        json!({"assigned": "RangeError", "grown": "RangeError"}),
        "{report}"
    );
    assert_eq!(
        report["written"],
        json!({
            "saved": {"tac:service:draft:/:visits": "2"},
            "session": {"tac:service:draft:/:draft": "\"chapter one\""},
        }),
        "{report}"
    );
    assert_eq!(
        report["afterReload"],
        json!({"visits": 2, "draft": "chapter one", "visit": 3}),
        "{report}"
    );
    assert_eq!(
        report["newTab"],
        json!({"visits": 3, "draft": ""}),
        "{report}"
    );
    assert_eq!(
        report["saved"],
        json!({"tac:service:draft:/:visits": "3"}),
        "{report}"
    );
}

one_test_per_case!(
    a_service_writes_its_local_state_to_page_storage_and_restores_it,
    async a_service_writes_its_local_state_to_page_storage_and_restores_it_in,
    all StoredStateProof::LANGUAGES,
    [dart => "dart", go => "go", rust => "rs"]
);
