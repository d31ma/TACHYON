#![allow(clippy::expect_used)]

use std::fs;
use std::process::{Command, Stdio};
use tachyon_contracts::{
    HandlerBody, HandlerBodyEncoding, HandlerRequest, HandlerResponse, HttpMethod,
};
use tachyon_core::{
    EnvironmentPolicy, HandlerCancellation, HandlerSource, HandlerSupervisor,
    HandlerSupervisorOptions,
};

struct RubyProject {
    root: tempfile::TempDir,
}

impl RubyProject {
    fn new() -> Self {
        Self {
            root: tempfile::tempdir().expect("project"),
        }
    }

    fn write(&self, relative: &str, contents: &str) {
        let path = self.root.path().join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        fs::write(path, contents).expect("source");
    }

    fn worker(&self, name: &str, contents: &str) -> HandlerSource {
        let relative = format!("server/workers/{name}.rb");
        self.write(&relative, contents);
        HandlerSource::discover(self.root.path(), &relative)
            .unwrap_or_else(|failure| panic!("{name}: {failure}"))
    }
}

fn ruby_available() -> bool {
    let found = Command::new("ruby")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    let required = cfg!(unix) && std::env::var_os("CI").is_some();
    assert!(
        found || !required,
        "ruby is required for the Ruby handler tests in CI"
    );
    if !found {
        eprintln!("skipping Ruby handler execution: ruby is unavailable");
    }
    found
}

fn request(id: &str, method: HttpMethod, body: Option<&str>) -> HandlerRequest {
    let mut request = HandlerRequest::route(id, "/contract", method);
    request.headers.insert(
        String::from("x-contract"),
        vec![String::from("shared"), String::from("second")],
    );
    request.body = body.map(|data| HandlerBody {
        encoding: HandlerBodyEncoding::Utf8,
        data: String::from(data),
    });
    request
}

async fn invoke(source: &HandlerSource, request: &HandlerRequest) -> HandlerResponse {
    invoke_with(HandlerSupervisorOptions::default(), source, request).await
}

async fn invoke_with(
    options: HandlerSupervisorOptions,
    source: &HandlerSource,
    request: &HandlerRequest,
) -> HandlerResponse {
    HandlerSupervisor::new(options)
        .expect("supervisor")
        .invoke(source, request, &HandlerCancellation::default())
        .await
        .expect("handler response")
}

fn body_json(response: &HandlerResponse) -> serde_json::Value {
    let body = response.body.as_ref().expect("successful body");
    serde_json::from_str(&body.data).expect("JSON handler result")
}

fn error_code(response: &HandlerResponse) -> String {
    response.error.as_ref().expect("error").code.clone()
}

#[tokio::test]
async fn a_ruby_worker_reads_the_request_and_keeps_stdout_off_the_protocol() {
    if !ruby_available() {
        return;
    }
    let project = RubyProject::new();
    let source = project.worker(
        "contract",
        r#"
class Handler
  def self.POST(request)
    puts "sideband-only"
    {
      method: request.method,
      headers: request.get_fields("x-contract"),
      body: request.body,
      unicode: "héllø 🌍"
    }
  end
end
"#,
    );
    let response = invoke(
        &source,
        &request("shared", HttpMethod::Post, Some("payload")),
    )
    .await;
    assert_eq!(response.status, 200, "{response:?}");
    let value = body_json(&response);
    assert_eq!(value["method"], "POST");
    assert_eq!(value["headers"], serde_json::json!(["shared", "second"]));
    assert_eq!(value["body"], "payload");
    assert_eq!(value["unicode"], "héllø 🌍");
}

#[tokio::test]
async fn ruby_authoring_failures_are_bounded_protocol_responses() {
    if !ruby_available() {
        return;
    }
    let project = RubyProject::new();
    let worker = project.worker(
        "failures",
        r#"
class Handler
  def self.GET(request)
    raise "bounded failure"
  end

  def self.PUT(request)
    Object.new
  end

  def POST(request)
    { "instance" => true }
  end
end
"#,
    );
    let missing = invoke(&worker, &request("missing", HttpMethod::Delete, None)).await;
    assert_eq!(missing.status, 405);
    assert_eq!(error_code(&missing), "TY2202");
    let failed = invoke(&worker, &request("failed", HttpMethod::Get, None)).await;
    assert_eq!(error_code(&failed), "TY2201");
    assert_eq!(failed.error.expect("error").message, "bounded failure");
    let instance = invoke(&worker, &request("instance", HttpMethod::Post, None)).await;
    assert_eq!(error_code(&instance), "TY2202");
    let unserializable = invoke(&worker, &request("object", HttpMethod::Put, None)).await;
    assert_eq!(error_code(&unserializable), "TY2203");
    let broken = project.worker(
        "broken",
        "class Handler\n  def self.GET(request)\n    return +\n  end\nend\n",
    );
    let syntax = invoke(&broken, &request("syntax", HttpMethod::Get, None)).await;
    assert_eq!(error_code(&syntax), "TY2201");
}

#[tokio::test]
async fn a_ruby_enumerator_streams_its_events_in_order() {
    if !ruby_available() {
        return;
    }
    let project = RubyProject::new();
    let source = project.worker(
        "events",
        r#"
class Handler
  def self.GET(request)
    Enumerator.new do |events|
      3.times { |index| events << { "index" => index } }
    end
  end
end
"#,
    );
    let supervisor =
        HandlerSupervisor::new(HandlerSupervisorOptions::default()).expect("supervisor");
    let (sender, mut receiver) = tokio::sync::mpsc::channel(8);
    supervisor
        .invoke_streaming(&source, &request("events", HttpMethod::Get, None), sender)
        .await
        .expect("stream");
    let mut events = Vec::new();
    while let Some(event) = receiver.recv().await {
        events.push(event.data);
    }
    assert_eq!(events, ["{\"index\":0}", "{\"index\":1}", "{\"index\":2}"]);
}

#[tokio::test]
async fn a_ruby_worker_sees_only_allowlisted_environment_variables() {
    if !ruby_available() {
        return;
    }
    let (name, expected) = ["CARGO_MANIFEST_DIR", "HOME", "USERPROFILE", "CI"]
        .iter()
        .find_map(|name| std::env::var(name).ok().map(|value| (*name, value)))
        .expect("test host needs one non-baseline environment variable");
    let project = RubyProject::new();
    let source = project.worker(
        "environment",
        r#"
class Handler
  def self.POST(request)
    name = request.body
    { "present" => ENV.key?(name), "value" => ENV[name] }
  end
end
"#,
    );
    let invocation = request("environment", HttpMethod::Post, Some(name));
    let denied = invoke(&source, &invocation).await;
    assert_eq!(body_json(&denied)["present"], false);
    let options = HandlerSupervisorOptions {
        environment: EnvironmentPolicy::from_names([name]).expect("allowlist"),
        ..HandlerSupervisorOptions::default()
    };
    let allowed = body_json(&invoke_with(options, &source, &invocation).await);
    assert_eq!(allowed["present"], true);
    assert_eq!(allowed["value"], expected);
}

#[tokio::test]
async fn a_ruby_worker_requires_plain_code_from_server_shared() {
    if !ruby_available() {
        return;
    }
    let project = RubyProject::new();
    project.write(
        "server/shared/Greeting.rb",
        "class Greeting\n  def self.text\n    \"hello from shared\"\n  end\nend\n",
    );
    let source = project.worker(
        "greeting",
        "require_relative \"../shared/Greeting\"\n\nclass Handler\n  def self.GET(request)\n    { \"text\" => Greeting.text }\n  end\nend\n",
    );
    let response = invoke(&source, &request("shared", HttpMethod::Get, None)).await;
    assert_eq!(
        body_json(&response)["text"],
        "hello from shared",
        "{response:?}"
    );
}

#[tokio::test]
async fn a_missing_ruby_runtime_is_ty2112_without_its_path() {
    let project = RubyProject::new();
    let source = project.worker(
        "runtime",
        "class Handler\n  def self.GET(request)\n    {}\n  end\nend\n",
    );
    let canary = "tachyon-ruby-does-not-exist-secret-canary";
    let mut options = HandlerSupervisorOptions::default();
    options.runtimes.ruby = canary.into();
    let failure = HandlerSupervisor::new(options)
        .expect("supervisor")
        .invoke(
            &source,
            &request("runtime", HttpMethod::Get, None),
            &HandlerCancellation::default(),
        )
        .await
        .expect_err("missing runtime");
    let rendered = failure.to_string();
    assert!(rendered.contains("TY2112"), "{rendered}");
    assert!(!rendered.contains(canary), "{rendered}");
}

fn layout_failure(relative: &str, contents: &str) -> String {
    let project = RubyProject::new();
    project.write(relative, contents);
    HandlerSource::discover(project.root.path(), relative)
        .expect_err("layout failure")
        .to_string()
}

#[test]
fn a_ruby_worker_holds_one_class_named_handler() {
    let two = layout_failure(
        "server/workers/two.rb",
        "class Handler\nend\n\nclass Helper\nend\n",
    );
    assert!(two.contains("another class 'Helper'"), "{two}");
    let empty = layout_failure("server/workers/empty.rb", "# no class here, it's empty\n");
    assert!(empty.contains("found no class declaration"), "{empty}");
    let indented = layout_failure(
        "server/workers/indent.rb",
        "class Handler\n  def self.GET(request)\n    {}\n  end\n  end\n",
    );
    assert!(indented.contains("same indentation"), "{indented}");
    let renamed = layout_failure(
        "server/workers/renamed.rb",
        "class Worker\n  def self.GET(request)\n    {}\n  end\nend\n",
    );
    assert!(
        renamed.contains("'Worker' instead of the 'Handler'"),
        "{renamed}"
    );
}

#[tokio::test]
async fn ruby_providers_receive_needs_fields_and_named_providers() {
    if !ruby_available() {
        return;
    }
    let project = RubyProject::new();
    project.write(
        "server/helpers/LedgerAmountsHelper.rb",
        "#[Helper]\n#[Lifetime(\"singleton\")]\nclass LedgerAmountsHelper\n  def subtotal\n    40\n  end\n\n  def surcharge\n    2\n  end\nend\n",
    );
    project.write(
        "server/services/LedgerService.rb",
        "#[Service]\nclass LedgerService\n  #[Needs]\n  # @return [LedgerAmountsHelper]\n  attr_reader :amounts\n\n  def subtotal\n    amounts.subtotal\n  end\nend\n",
    );
    project.write(
        "server/helpers/LedgerHelper.rb",
        "#[Helper]\n#[Lifetime(\"transient\")]\nclass LedgerHelper\n  #[Needs]\n  attr_reader :ledger_amounts_helper\n\n  def settle(subtotal)\n    subtotal + ledger_amounts_helper.surcharge\n  end\nend\n",
    );
    project.write(
        "server/controllers/LedgerController.rb",
        "#[RequestRoute(\"/ledger\")]\n#[Public]\nclass LedgerController\n  #[Needs]\n  #[Named(\"LedgerService\")]\n  attr_reader :service\n  #[Needs]\n  #[Named(\"LedgerHelper\")]\n  attr_reader :settlement\n\n  #[GetRoute]\n  def get_ledger\n    { \"total\" => settlement.settle(service.subtotal) }\n  end\nend\n",
    );
    let source = HandlerSource::discover(
        project.root.path(),
        "server/controllers/LedgerController.rb",
    )
    .unwrap_or_else(|failure| panic!("{failure}"));
    let mut request = HandlerRequest::route("ledger", "/ledger", HttpMethod::Get);
    request.arguments = Some(Vec::new());
    let response = invoke(&source, &request).await;
    assert_eq!(
        body_json(&response),
        serde_json::json!({"total": 42}),
        "{response:?}"
    );
}

#[tokio::test]
async fn a_ruby_needs_all_class_receives_every_attr_field() {
    if !ruby_available() {
        return;
    }
    let project = RubyProject::new();
    project.write(
        "server/helpers/LedgerAmountsHelper.rb",
        "#[Helper]\nclass LedgerAmountsHelper\n  def subtotal\n    40\n  end\nend\n",
    );
    project.write(
        "server/services/LedgerService.rb",
        "#[Service]\n#[NeedsAll]\nclass LedgerService\n  # @return [LedgerAmountsHelper]\n  attr_reader :amounts\n\n  def subtotal\n    amounts.subtotal + 2\n  end\nend\n",
    );
    project.write(
        "server/controllers/LedgerController.rb",
        "#[RequestRoute(\"/ledger\")]\n#[Public]\n#[NeedsAll]\nclass LedgerController\n  #[Needs]\n  #[Named(\"LedgerService\")]\n  attr_reader :service\n  attr_reader :ledger_amounts_helper\n\n  #[GetRoute]\n  def get_ledger\n    { \"total\" => service.subtotal, \"store\" => ledger_amounts_helper.subtotal }\n  end\nend\n",
    );
    let source = HandlerSource::discover(
        project.root.path(),
        "server/controllers/LedgerController.rb",
    )
    .unwrap_or_else(|failure| panic!("{failure}"));
    let mut request = HandlerRequest::route("ledger", "/ledger", HttpMethod::Get);
    request.arguments = Some(Vec::new());
    let response = invoke(&source, &request).await;
    assert_eq!(
        body_json(&response),
        serde_json::json!({"total": 42, "store": 40}),
        "{response:?}"
    );
}

#[test]
fn a_ruby_needs_all_field_without_a_provider_fails_the_build() {
    let failure = layout_failure(
        "server/controllers/LedgerController.rb",
        "#[RequestRoute(\"/ledger\")]\n#[Public]\n#[NeedsAll]\nclass LedgerController\n  attr_reader :missing_store\n\n  #[GetRoute]\n  def get_ledger\n    missing_store.subtotal\n  end\nend\n",
    );
    assert!(
        failure.contains("LedgerController needs field 'missing_store' of type MissingStore"),
        "{failure}"
    );
}

#[tokio::test]
async fn a_ruby_route_that_yields_streams_each_value() {
    if !ruby_available() {
        return;
    }
    let project = RubyProject::new();
    project.write(
        "server/services/TicketService.rb",
        "#[Service]\nclass TicketService\n  def levels\n    [1, 2, 3]\n  end\nend\n",
    );
    project.write(
        "server/controllers/EventsController.rb",
        "#[RequestRoute(\"/events\")]\n#[Public]\nclass EventsController\n  #[Needs]\n  attr_reader :ticket_service\n\n  #[StreamRoute]\n  def stream_events\n    each_level(ticket_service.levels) { |event| yield event }\n  end\n\n  private\n\n  def each_level(levels)\n    levels.each do |level|\n      yield({ \"level\" => level })\n    end\n  end\nend\n",
    );
    let source = HandlerSource::discover(
        project.root.path(),
        "server/controllers/EventsController.rb",
    )
    .unwrap_or_else(|failure| panic!("{failure}"));
    assert!(source.streams_method(HttpMethod::Get));
    let mut request = HandlerRequest::route("events", "/events", HttpMethod::Get);
    request.arguments = Some(Vec::new());
    let supervisor =
        HandlerSupervisor::new(HandlerSupervisorOptions::default()).expect("supervisor");
    let (sender, mut receiver) = tokio::sync::mpsc::channel(8);
    supervisor
        .invoke_streaming(&source, &request, sender)
        .await
        .expect("stream");
    let mut events = Vec::new();
    while let Some(event) = receiver.recv().await {
        events.push(event.data);
    }
    assert_eq!(events, ["{\"level\":1}", "{\"level\":2}", "{\"level\":3}"]);
}
