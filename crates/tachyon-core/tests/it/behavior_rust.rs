#![allow(clippy::expect_used)]

use tachyon_contracts::{HandlerRequest, HttpMethod};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
};

use crate::support::behavior_layout;

use crate::support::authored_behaviors;

#[tokio::test]
async fn all_eight_rust_behavior_annotations_execute() {
    exercise(None).await;
}

#[tokio::test]
async fn rust_behavior_constructor_composes_with_yon_injection() {
    let source = r#"
#[yon::Service()]
struct Counter { value: i32 }
impl Counter { fn new() -> Self { Self { value: 42 } } }
#[RequestRoute("/products")]
#[Public]
#[NeedsAll]
#[yon::Needs(Counter)]
struct ProductsController { counter: std::sync::Arc<Counter> }
impl ProductsController {
    #[GetRoute]
    fn get_products(&self) -> http::Response<Vec<u8>> {
        assert_eq!(self.counter.value, 42);
        let body = CatalogChecksHelper::exercise().expect("behaviors").to_json(1_048_576).expect("JSON");
        http::Response::builder()
            .header("content-type", "application/json")
            .body(body.into_bytes())
            .unwrap_or_default()
    }
}
"#;
    exercise(Some(source)).await;
}

async fn exercise(authored: Option<&str>) {
    let project = tempfile::tempdir().expect("project");
    let relative = "server/controllers/ProductsController.rs";
    authored_behaviors::write(project.path(), "rs", "server", "rs");
    if let Some(authored) = authored {
        behavior_layout::write(project.path(), relative, authored);
    }
    let source = HandlerSource::discover(project.path(), relative).expect("annotated discovery");
    let supervisor =
        HandlerSupervisor::new(HandlerSupervisorOptions::default()).expect("supervisor");
    let response = supervisor
        .invoke(
            &source,
            &HandlerRequest::route("rust_behaviors", "/products", HttpMethod::Get),
            &HandlerCancellation::default(),
        )
        .await
        .expect("invocation");
    assert!(response.error.is_none(), "{:?}", response.error);
    let body: serde_json::Value =
        serde_json::from_str(&response.body.expect("body").data).expect("JSON");
    assert_eq!(body["first"], body["cached"]);
    assert_eq!(body["first"]["page"]["offset"], 10);
    assert_eq!(body["fresh"]["calls"], 2);
    assert_eq!(body["remote"], "/products");
    assert_eq!(body["saved"]["operation"], "save");
    assert_eq!(body["found"]["table"], "products");
    assert_eq!(body["deleted"]["operation"], "delete");
}
