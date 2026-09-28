#![allow(clippy::expect_used)]
use tachyon_contracts::{HandlerRequest, HttpMethod};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
};

use crate::support::authored_behaviors;

#[tokio::test]
async fn python_php_and_ruby_execute_all_shared_annotations() {
    for extension in ["py", "php", "rb"] {
        let runtime = match extension {
            "py" => "python3",
            "php" => "php-cgi",
            _ => "ruby",
        };
        if std::process::Command::new(runtime)
            .arg("--version")
            .output()
            .is_err()
        {
            eprintln!(
                "skipping {extension}: {runtime} is unavailable; required polyglot CI preflights every runtime"
            );
            continue;
        }
        assert_shared_annotations(extension).await;
    }
}

async fn assert_shared_annotations(extension: &str) {
    let project = tempfile::tempdir().expect("project");
    let path = format!("server/controllers/ProductsController.{extension}");
    authored_behaviors::write(project.path(), extension, "server", extension);
    let source = HandlerSource::discover(project.path(), &path).expect("discovery");
    let supervisor =
        HandlerSupervisor::new(HandlerSupervisorOptions::default()).expect("supervisor");
    let response = supervisor
        .invoke(
            &source,
            &HandlerRequest::route("polyglot_behavior", "/products", HttpMethod::Get),
            &HandlerCancellation::default(),
        )
        .await
        .expect("invocation");
    assert!(
        response.error.is_none(),
        "{extension}: {:?}",
        response.error
    );
    let result: serde_json::Value =
        serde_json::from_str(&response.body.expect("body").data).expect("JSON");
    assert_eq!(result["first"], result["cached"], "{extension}");
    assert_eq!(result["fresh"]["calls"], 2, "{extension}");
    assert_eq!(result["first"]["page"]["offset"], 10, "{extension}");
    assert_eq!(result["remote"]["path"], "/products", "{extension}");
    assert_eq!(result["saved"]["operation"], "save", "{extension}");
    assert_eq!(result["found"]["table"], "products", "{extension}");
    assert_eq!(result["deleted"]["operation"], "delete", "{extension}");
    assert_eq!(result["cancelled"], true, "{extension}");
    assert_eq!(result["authorizations"], 12, "{extension}");
}
