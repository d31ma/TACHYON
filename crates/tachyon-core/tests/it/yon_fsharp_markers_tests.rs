#![allow(clippy::expect_used)]

use std::path::Path;
use std::time::Duration;
use tachyon_core::{DevServer, DevServerOptions, ProjectDiscovery};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpStream;

struct FsharpMarkers;

impl FsharpMarkers {
    const FILES: [(&'static str, &'static str); 14] = [
        (
            "server/application.json",
            r#"{"active":{"profile":"staging"},"catalog":{"page":{"size":7},"cache":{"enabled":true}}}"#,
        ),
        (
            "server/setups/OrdersSetup.fs",
            r"open System
open System.Threading.Tasks

[<Setup>]
type OrdersSetup() =
    static member configure(execution: BehaviorExecution) : BehaviorAdapters =
        let adapters = BehaviorAdapters()
        adapters.authorize <- Func<_, _, _>(fun call current -> Task.FromResult(true))
        adapters
",
        ),
        (
            "server/services/OrdersService.fs",
            r#"open System
open System.Threading.Tasks

[<Service>]
[<Description("Counts the attempts of every order")>]
type OrdersService() =
    let firstWorkingAttempt = 3
    let mutable attempts = 0
    let mutable running = 0
    let mutable most = 0

    member val behaviors : BehaviorContext = Behaviors.create(typeof<OrdersSetup>, "orders", new BehaviorSignal())

    [<Retry({"maxRetries":3,"delayMs":10,"multiplier":2,"maxDelayMs":200,"jitterMs":5})>]
    member this.submit(id: BehaviorJSON) : Task<BehaviorJSON> =
        attempts <- attempts + 1
        this.FailFirstTwoAttempts()
        Task.FromResult(BehaviorJSON.Object(dict [ "id", id; "attempts", BehaviorJSON.From(attempts) ]))

    [<MaxAtOnce({"limit":1,"policy":"reject"})>]
    member this.hold(id: BehaviorJSON) : Task<BehaviorJSON> =
        task {
            do! Task.Delay(500)
            return id
        }

    [<MaxAtOnce({"limit":1,"policy":"block"})>]
    member this.queue(id: BehaviorJSON) : Task<BehaviorJSON> =
        task {
            running <- running + 1
            most <- max most running
            do! Task.Delay(100)
            running <- running - 1
            return BehaviorJSON.From(most)
        }

    member private this.FailFirstTwoAttempts() : unit =
        if attempts < firstWorkingAttempt then
            raise (InvalidOperationException("the payment gateway is unavailable"))
"#,
        ),
        (
            "server/models/OrderReport.fs",
            "type OrderReport =\n    { Submitted: System.Text.Json.Nodes.JsonNode\n      Held: System.Text.Json.Nodes.JsonNode\n      Rejected: string\n      Queued: System.Text.Json.Nodes.JsonNode\n      Description: string }\n",
        ),
        (
            "server/controllers/OrdersController.fs",
            r#"open System.Reflection
open System.Text.Json.Nodes
open System.Threading.Tasks
open Models

[<RequestRoute("/orders")>]
[<Public>]
type OrdersController(orders: OrdersService) =
    [<GetRoute>]
    member this.GetOrders() : OrderReport = this.BuildReport().Result

    member private this.BuildReport() : Task<OrderReport> =
        task {
            let! submitted = orders.submit(BehaviorJSON.From("A-1"))
            let first = orders.hold(BehaviorJSON.From("first"))
            let! rejected =
                task {
                    try
                        let! _ = orders.hold(BehaviorJSON.From("second"))
                        return "accepted"
                    with :? BehaviorError as error ->
                        return error.Code
                }
            let! held = first
            let! queued = Task.WhenAll([| orders.queue(BehaviorJSON.From("left")); orders.queue(BehaviorJSON.From("right")) |])
            let node (value: BehaviorJSON) = JsonNode.Parse(value.ToString())
            return
                { Submitted = node submitted
                  Held = node held
                  Rejected = rejected
                  Queued = node (BehaviorJSON.Array(queued))
                  Description = this.Described() }
        }

    member private this.Described() : string =
        let read = typeof<Behaviors>.GetMethod("TypePolicies", BindingFlags.NonPublic ||| BindingFlags.Static)
        let policies = read.Invoke(null, [| box typeof<OrdersService> |]) :?> BehaviorJSON
        policies.Get("description").Text
"#,
        ),
        (
            "server/setups/CatalogSetup.fs",
            r#"[<Setup>]
[<Settings({"prefix":"YON_CATALOG_"})>]
[<WhenProfile("development")>]
[<WhenSetting({"name":"YON_CATALOG_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})>]
type CatalogSetup() =
    [<Min(1)>]
    [<Max(100)>]
    member val pageSize : int = 20 with get, set

    member val cacheEnabled : bool = false with get, set

    static member configure(execution: BehaviorExecution) : BehaviorAdapters = BehaviorAdapters()
"#,
        ),
        (
            "server/models/Endpoint.fs",
            "type Endpoint() =\n    [<Setting(\"${yon.active.profile:development}\")>]\n    member val profile : string = \"\" with get, set\n",
        ),
        (
            "server/models/Settings.fs",
            "type Settings =\n    { Profile: string\n      PageSize: int\n      CacheEnabled: bool\n      Development: string\n      Active: string\n      CacheOff: string }\n",
        ),
        (
            "server/controllers/SettingsController.fs",
            r#"open System
open Models

[<RequestRoute("/settings")>]
[<Public>]
type SettingsController() =
    static let development = "development"

    [<GetRoute>]
    member this.GetSettings() : Settings =
        let application = BehaviorJSON.Parse(Environment.GetEnvironmentVariable("YON_APPLICATION_JSON"))
        let catalog = Behaviors.properties(typeof<CatalogSetup>, application)
        let endpoint = Behaviors.values(typeof<Endpoint>, application)
        let profile = endpoint.Get("profile").Text
        let cacheOff = BehaviorJSON.Parse("{\"yon\":{\"catalog\":{\"page\":{\"size\":7},\"cache\":{\"enabled\":false}}}}")
        { Profile = profile
          PageSize = int (catalog.Get("pageSize").Integer)
          CacheEnabled = catalog.Get("cacheEnabled").Boolean
          Development = SettingsController.Select(application, development)
          Active = SettingsController.Select(application, profile)
          CacheOff = SettingsController.Select(cacheOff, development) }

    static member private Select(application: BehaviorJSON, profile: string) : string =
        try
            use signal = new BehaviorSignal()
            use context = Behaviors.create(typeof<CatalogSetup>, "public-catalog", signal, application, profiles = [| profile |])
            "selected"
        with :? BehaviorError as error ->
            error.Code
"#,
        ),
        (
            "server/models/Item.fs",
            "type Item = { Name: string; Level: int64 }\n",
        ),
        (
            "server/controllers/ItemsController.fs",
            r#"open Models

[<RequestRoute("/items")>]
[<Public>]
type ItemsController() =
    [<GetRoute("/{id}")>]
    member this.GetItem([<FromPath>] id: int64) : Item =
        { Name = "read"; Level = id }

    [<PostRoute>]
    [<ResponseStatus(201)>]
    member this.PostItem([<FromBody>] item: Item) : Item =
        item

    [<PatchRoute("/{id}")>]
    member this.PatchItem([<FromPath>] id: int64, [<FromBody>] item: Item) : Item =
        { item with Level = id }

    [<DeleteRoute("/{id}")>]
    member this.DeleteItem([<FromPath>] id: int64) : Item =
        { Name = "deleted"; Level = id }
"#,
        ),
        (
            "server/errors/OrderNotFound.fs",
            "type OrderNotFound(message: string) =\n    inherit System.Exception(message)\n",
        ),
        (
            "server/models/Problem.fs",
            "type Problem = { Code: string }\n",
        ),
        (
            "server/controllers/OrdersErrorHandler.fs",
            r#"open Models

[<ErrorHandler>]
type OrdersErrorHandler() =
    [<HandlesError>]
    [<ResponseStatus(404)>]
    member this.Missing(error: OrderNotFound) : Problem =
        { Code = "order-not-found" }
"#,
        ),
    ];

    const RECEIPTS: (&'static str, &'static str) = (
        "server/controllers/ReceiptsController.fs",
        r#"open Models

[<RequestRoute("/receipts")>]
[<Public>]
type ReceiptsController() =
    static let privateErrorSentinel = "PRIVATE_ERROR_SENTINEL"

    [<GetRoute>]
    member this.GetReceipt() : Problem =
        raise (OrderNotFound(privateErrorSentinel))

    [<GetRoute("/broken")>]
    member this.GetBroken() : Problem =
        raise (System.InvalidOperationException(privateErrorSentinel))
"#,
    );

    const REQUESTS: [(&'static str, &'static str, &'static str); 10] = [
        ("GET", "/orders", ""),
        ("GET", "/settings", ""),
        ("GET", "/items/5", ""),
        ("POST", "/items", r#"{"name":"lamp","level":2}"#),
        ("PATCH", "/items/5", r#"{"name":"lamp","level":2}"#),
        ("DELETE", "/items/5", ""),
        ("PUT", "/items/5", r#"{"name":"lamp","level":2}"#),
        ("GET", "/receipts", ""),
        ("GET", "/receipts/broken", ""),
        ("GET", "/api.json", ""),
    ];

    fn dotnet_available() -> bool {
        std::process::Command::new("dotnet")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
    }

    fn write(root: &Path) {
        for (relative, source) in Self::FILES.into_iter().chain([Self::RECEIPTS]) {
            let target = root.join(relative);
            std::fs::create_dir_all(target.parent().expect("parent")).expect("folders");
            std::fs::write(target, source).expect("source");
        }
    }

    async fn send(port: u16, method: &str, path: &str, body: &str) -> String {
        let mut socket = TcpStream::connect(("127.0.0.1", port))
            .await
            .expect("connect");
        let length = body.len();
        let request = format!(
            "{method} {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n{body}"
        );
        socket.write_all(request.as_bytes()).await.expect("request");
        let mut response = Vec::new();
        tokio::time::timeout(Duration::from_mins(5), socket.read_to_end(&mut response))
            .await
            .expect("response deadline")
            .expect("response");
        String::from_utf8(response).expect("UTF-8 response")
    }

    async fn responses(root: &Path) -> Vec<String> {
        let project = ProjectDiscovery::discover(root).expect("discover");
        let options = DevServerOptions {
            port: 0,
            watch: false,
            ..DevServerOptions::default()
        };
        let server = DevServer::bind_project(&project, &options)
            .await
            .expect("bind");
        let port = server.address().port();
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let running = tokio::spawn(server.run_until(async {
            let _ = stopped.await;
        }));
        let mut responses = Vec::new();
        for (method, path, body) in Self::REQUESTS {
            responses.push(Self::send(port, method, path, body).await);
        }
        stop.send(()).expect("stop");
        running.await.expect("task").expect("shutdown");
        responses
    }

    fn answer(response: &str, status: u16) -> serde_json::Value {
        assert!(
            response.starts_with(&format!("HTTP/1.1 {status}")),
            "{response}"
        );
        let body = response.split_once("\r\n\r\n").expect("body").1;
        serde_json::from_str(body).unwrap_or_else(|error| panic!("{error}: {response}"))
    }
}

#[tokio::test]
async fn fsharp_markers_change_what_a_running_yon_server_answers() {
    if !FsharpMarkers::dotnet_available() {
        eprintln!("skipping: dotnet is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    FsharpMarkers::write(root.path());
    let responses = FsharpMarkers::responses(root.path()).await;

    let orders = FsharpMarkers::answer(&responses[0], 200);
    assert_eq!(
        orders["submitted"],
        serde_json::json!({"id": "A-1", "attempts": 3}),
        "{orders}"
    );
    assert_eq!(orders["held"], "first", "{orders}");
    assert_eq!(orders["rejected"], "OPERATION_LIMIT", "{orders}");
    assert_eq!(orders["queued"], serde_json::json!([1, 1]), "{orders}");
    assert_eq!(
        orders["description"], "Counts the attempts of every order",
        "{orders}"
    );

    let settings = FsharpMarkers::answer(&responses[1], 200);
    assert_eq!(
        settings,
        serde_json::json!({
            "profile": "staging",
            "pageSize": 7,
            "cacheEnabled": true,
            "development": "selected",
            "active": "inactive-configuration",
            "cacheOff": "inactive-configuration"
        })
    );

    assert_eq!(
        FsharpMarkers::answer(&responses[2], 200),
        serde_json::json!({"name": "read", "level": 5})
    );
    assert_eq!(
        FsharpMarkers::answer(&responses[3], 201),
        serde_json::json!({"name": "lamp", "level": 2})
    );
    assert_eq!(
        FsharpMarkers::answer(&responses[4], 200),
        serde_json::json!({"name": "lamp", "level": 5})
    );
    assert_eq!(
        FsharpMarkers::answer(&responses[5], 200),
        serde_json::json!({"name": "deleted", "level": 5})
    );
    assert!(responses[6].starts_with("HTTP/1.1 405"), "{}", responses[6]);

    assert_eq!(
        FsharpMarkers::answer(&responses[7], 404),
        serde_json::json!({"code": "order-not-found"})
    );
    assert!(responses[8].starts_with("HTTP/1.1 500"), "{}", responses[8]);
    for response in &responses[7..9] {
        assert!(!response.contains("PRIVATE_ERROR_SENTINEL"), "{response}");
    }

    let schema = FsharpMarkers::answer(&responses[9], 200);
    let receipts = schema["routes"]
        .as_array()
        .expect("routes")
        .iter()
        .find(|route| route["path"] == "/receipts")
        .unwrap_or_else(|| panic!("no /receipts route in {schema}"));
    let errors = receipts["methods"]["GET"]["errors"].to_string();
    assert!(
        errors.contains("OrderNotFound") && errors.contains("404"),
        "{receipts}"
    );
}
