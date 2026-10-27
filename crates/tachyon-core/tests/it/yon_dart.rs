#![allow(clippy::expect_used)]
use tachyon_contracts::{HandlerRequest, HttpMethod};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
};

fn write(root: &std::path::Path, relative: &str, source: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("directories");
    std::fs::write(path, source).expect("source");
}

fn dart_available() -> bool {
    let found = crate::support::program_probe::ProgramProbe::answers("dart", "--version");
    crate::support::toolchain::Toolchain::checked("dart", found)
}

async fn invoke(root: &std::path::Path, method: HttpMethod) -> tachyon_contracts::HandlerResponse {
    let source = HandlerSource::discover(root, "server/controllers/GreetController.dart")
        .expect("a Dart controller is a supported source");
    HandlerSupervisor::new(HandlerSupervisorOptions::default())
        .expect("supervisor")
        .invoke(
            &source,
            &HandlerRequest::route("dart", "/greet", method),
            &HandlerCancellation::default(),
        )
        .await
        .expect("invocation")
}

#[tokio::test]
async fn a_dart_controller_answers_the_verbs_it_declares() {
    if !dart_available() {
        eprintln!("skipping: dart is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    write_plain_controller(root.path());

    let get = invoke(root.path(), HttpMethod::Get).await;
    assert!(get.error.is_none(), "{:?}", get.error);
    assert_eq!(get.status, 200);
    let body: serde_json::Value =
        serde_json::from_str(&get.body.expect("GET body").data).expect("JSON");
    assert_eq!(body, serde_json::json!({"greeting": "hello", "largest": 3}));

    let post = invoke(root.path(), HttpMethod::Post).await;
    assert_eq!(post.status, 201, "{post:?}");
    assert_eq!(
        post.headers.get("location").map(Vec::as_slice),
        Some(&[String::from("/greet/1")][..]),
        "{post:?}"
    );

    let put = invoke(root.path(), HttpMethod::Put).await;
    assert_eq!(
        put.status, 405,
        "an undeclared verb is not dispatched: {put:?}"
    );
}

#[tokio::test]
async fn a_dart_controller_composes_its_role_files() {
    if !dart_available() {
        eprintln!("skipping: dart is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    write_controller_roles(root.path());
    let get = invoke(root.path(), HttpMethod::Get).await;
    assert!(get.error.is_none(), "{:?}", get.error);
    let body: serde_json::Value =
        serde_json::from_str(&get.body.expect("GET body").data).expect("JSON");
    assert_eq!(
        body,
        serde_json::json!({"message": "hello, dart (4)", "pi": true})
    );
}

fn write_plain_controller(root: &std::path::Path) {
    write(
        root,
        "server/controllers/GreetController.dart",
        concat!(
            "import 'dart:math';\n",
            "import 'package:shelf/shelf.dart';\n\n",
            "@RequestRoute(\"/greet\")\n@Public\n",
            "class GreetController {\n",
            "  @GetRoute\n",
            "  Object? getGreeting(Request request) {\n",
            "    return {'greeting': 'hello', 'largest': max(2, 3)};\n",
            "  }\n\n",
            "  @PostRoute\n",
            "  Future<Response> postGreeting(Request request) async {\n",
            "    return Response(201, body: '{\"created\":true}', headers: {'content-type': 'application/json', 'location': '/greet/1'});\n",
            "  }\n",
            "}\n",
        ),
    );
}

fn write_controller_roles(root: &std::path::Path) {
    write(
        root,
        "server/services/GreetingService.dart",
        concat!(
            "import 'dart:math';\n\n",
            "@Service\n",
            "class GreetingService {\n",
            "  String greet(String name) {\n",
            "    return 'hello, $name (${sqrt(16).toInt()})';\n",
            "  }\n",
            "}\n",
        ),
    );
    write(
        root,
        "server/controllers/GreetController.dart",
        concat!(
            "import 'dart:math';\n\n",
            "@RequestRoute(\"/greet\")\n@Public\n",
            "class GreetController {\n",
            "  @Needs\n",
            "  late final GreetingService greetingService;\n\n",
            "  @GetRoute\n",
            "  Future<Object?> getGreeting() async {\n",
            "    return {'message': greetingService.greet('dart'), 'pi': pi > 3};\n",
            "  }\n",
            "}\n",
        ),
    );
}
