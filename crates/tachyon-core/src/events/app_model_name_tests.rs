#![allow(clippy::expect_used)]

use super::{EventModule, EventStream, describe, extract};
use serde_json::json;
use std::path::Path;

struct ShadowingProject {
    root: tempfile::TempDir,
}

impl ShadowingProject {
    const JS_MODEL: &str =
        "/**\n * @typedef {Object} UploadFinished\n * @property {string} note\n */\nexport {}\n";
    const TS_MODEL: &str = "export interface UploadFinished {\n  note: string\n}\n";
    const JS_SERVICE: &str = "class NoteService {\n/** @param {UploadFinished} finished */\n@OnEvent\nfinished(finished) {}\n}";
    const TS_SERVICE: &str =
        "class NoteService {\n@OnEvent\nfinished(finished: UploadFinished) {}\n}";

    fn new() -> Self {
        Self {
            root: tempfile::tempdir().expect("root"),
        }
    }

    fn write(&self, relative: &str, text: &str) {
        let path = self.root.path().join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        std::fs::write(path, text).expect("write");
    }

    fn module(path: &Path, source: &str) -> EventModule {
        extract(path, source, "tac")
            .expect("parse")
            .expect("events")
    }

    fn described(&self, relative: &str, source: &str) -> Result<EventModule, String> {
        let path = Path::new(relative);
        let mut module = Self::module(path, source);
        describe(&mut module, path, self.root.path(), "client")
            .map(|()| module)
            .map_err(|failure| failure.to_string())
    }

    fn listener(parameter: &str) -> String {
        format!("class NoteService {{\n@OnEvent\nheard(event: {parameter}) {{}}\n}}\n")
    }

    fn model(name: &str) -> String {
        format!("export interface {name} {{\n  note: string\n}}\n")
    }
}

#[test]
fn a_javascript_app_model_named_like_a_built_in_event_keeps_its_own_fields() {
    let project = ShadowingProject::new();
    project.write(
        "client/models/UploadFinished.js",
        ShadowingProject::JS_MODEL,
    );
    let module = project
        .described(
            "client/services/NoteService.js",
            ShadowingProject::JS_SERVICE,
        )
        .expect("described");
    assert_eq!(
        module.types[0].listeners[0].models,
        Some(json!({"UploadFinished": {"open": false, "fields": {"note": "!string"}}}))
    );
}

#[test]
fn a_typescript_app_model_named_like_a_built_in_event_keeps_its_own_fields() {
    let project = ShadowingProject::new();
    project.write(
        "client/models/UploadFinished.ts",
        ShadowingProject::TS_MODEL,
    );
    let module = project
        .described(
            "client/services/NoteService.ts",
            ShadowingProject::TS_SERVICE,
        )
        .expect("described");
    assert_eq!(
        module.types[0].listeners[0].models,
        Some(json!({"UploadFinished": {"open": false, "fields": {"note": "!string"}}}))
    );
}

#[test]
fn a_built_in_event_still_reaches_a_listener_when_the_app_has_no_model_of_that_name() {
    let project = ShadowingProject::new();
    let module = project
        .described(
            "client/services/NoteService.ts",
            ShadowingProject::TS_SERVICE,
        )
        .expect("described");
    let fields = &module.types[0].listeners[0]
        .models
        .as_ref()
        .expect("models")["UploadFinished"]["fields"];
    assert_eq!(fields["upload"], json!("!string"), "{fields}");
}

#[test]
fn a_listener_of_an_app_model_fails_when_the_granted_api_sends_the_built_in_event() {
    let project = ShadowingProject::new();
    project.write(
        "client/models/UploadFinished.ts",
        ShadowingProject::TS_MODEL,
    );
    project.write(
        "client/manifest.json",
        r#"{"tac":{"browserApis":["files"]}}"#,
    );
    let error = project
        .described(
            "client/services/NoteService.ts",
            ShadowingProject::TS_SERVICE,
        )
        .expect_err("the files grant also sends UploadFinished");
    assert!(error.contains("TY1376"), "{error}");
    assert!(error.contains("'UploadFinished'"), "{error}");
    assert!(error.contains("NoteService.finished"), "{error}");
    assert!(error.contains("grants \"files\""), "{error}");
    assert!(error.contains("Rename"), "{error}");
}

#[test]
fn a_listener_of_an_app_model_builds_when_another_api_is_granted() {
    let project = ShadowingProject::new();
    project.write(
        "client/models/UploadFinished.ts",
        ShadowingProject::TS_MODEL,
    );
    project.write(
        "client/manifest.json",
        r#"{"tac":{"browserApis":["clipboard"]}}"#,
    );
    project
        .described(
            "client/services/NoteService.ts",
            ShadowingProject::TS_SERVICE,
        )
        .expect("clipboard does not send UploadFinished");
}

#[test]
fn a_listener_of_an_app_model_fails_when_the_page_always_sends_the_built_in_event() {
    let project = ShadowingProject::new();
    project.write(
        "client/models/ConnectivityChanged.ts",
        &ShadowingProject::model("ConnectivityChanged"),
    );
    let source = ShadowingProject::listener("ConnectivityChanged");
    let error = project
        .described("client/services/NoteService.ts", &source)
        .expect_err("the page always sends ConnectivityChanged");
    assert!(error.contains("TY1376"), "{error}");
    assert!(error.contains("the page always sends it"), "{error}");
}

#[test]
fn a_socket_event_model_of_the_app_fails_only_in_a_service_that_holds_a_socket() {
    let project = ShadowingProject::new();
    project.write(
        "client/models/SocketOpened.ts",
        &ShadowingProject::model("SocketOpened"),
    );
    let source = ShadowingProject::listener("SocketOpened");
    let path = Path::new("client/services/NoteService.ts");
    project
        .described("client/services/NoteService.ts", &source)
        .expect("no socket client, so no built-in SocketOpened");
    let mut module = ShadowingProject::module(path, &source);
    module.types[0].streams.push(EventStream {
        method: String::from("receive"),
        path: String::from("/chat"),
        event: String::from("SocketOpened"),
        socket: true,
        models: None,
        topic: false,
    });
    let error = describe(&mut module, path, project.root.path(), "client")
        .expect_err("the socket sends SocketOpened")
        .to_string();
    assert!(error.contains("socket client"), "{error}");
}
