#![allow(clippy::expect_used)]

use tachyon_contracts::{HandlerRequest, HttpMethod};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
};

use crate::support::behavior_layout;

use crate::support::authored_behaviors;

#[tokio::test]
async fn csharp_annotations_execute_in_plain_and_injected_yon_endpoints() {
    if std::process::Command::new("dotnet")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    for injected in [false, true] {
        exercise(injected).await;
    }
}

fn injected_source() -> &'static str {
    r#"
[RequestRoute("/products")]
[Public]
[NeedsAll]
[Needs(typeof(CounterService))]
class ProductsController {
    readonly CounterService _counter;
    [GetRoute]
    public HttpResponseMessage GetProducts(HttpRequestMessage request) {
        RequireCounter();
        RequireGet(request);
        return new HttpResponseMessage(System.Net.HttpStatusCode.OK) { Content = new StringContent(CatalogExercisesDelegate.run().GetAwaiter().GetResult().ToString(), System.Text.Encoding.UTF8, "application/json") };
    }
    private void RequireCounter() {
        if (_counter.value != 42) throw new System.Exception("constructor injection failed");
    }
    private static void RequireGet(HttpRequestMessage request) {
        if (request.Method != HttpMethod.Get) throw new System.Exception("constructor injection failed");
    }
}
"#
}

async fn exercise(injected: bool) {
    let project = tempfile::tempdir().expect("project");
    let relative = "server/controllers/ProductsController.cs";
    authored_behaviors::write(project.path(), "cs", "server", "cs");
    if injected {
        behavior_layout::write(
            project.path(),
            "server/services/CounterService.cs",
            "[Service]\nclass CounterService { public int value = 42; }\n",
        );
        behavior_layout::write(project.path(), relative, injected_source());
    }
    let source = HandlerSource::discover(project.path(), relative).expect("C# annotated discovery");
    let supervisor =
        HandlerSupervisor::new(HandlerSupervisorOptions::default()).expect("supervisor");
    let response = supervisor
        .invoke(
            &source,
            &HandlerRequest::route("csharp_behaviors", "/products", HttpMethod::Get),
            &HandlerCancellation::default(),
        )
        .await
        .expect("C# invocation");
    assert!(response.error.is_none(), "{:?}", response.error);
    let body: serde_json::Value =
        serde_json::from_str(&response.body.expect("body").data).expect("JSON");
    assert_eq!(body["first"], body["cached"]);
    assert_eq!(body["first"]["offset"], 6);
    assert_eq!(body["fresh"]["calls"], 2);
    assert_eq!(body["page"]["offset"], 6);
    assert_eq!(body["remote"]["transport"], true);
    assert_eq!(body["saved"]["operation"], "save");
    assert_eq!(body["found"]["table"], "products");
    assert_eq!(body["deleted"]["id"], 4);
}
