#![allow(clippy::expect_used)]

use super::java_tests::{run_driver, write};
use super::kotlin_tests::kotlin_web_ready;
use super::{BuildJobs, WebLanguage, build_all};
use std::path::Path;

struct HttpErrorExample;

impl HttpErrorExample {
    const EXAMPLE: &str = "../../website/content/docs/calling-the-server/http-error-bodies/client";
    const FILES: [&str; 6] = [
        "models/OrderInput",
        "models/Problem",
        "models/ProblemEntry",
        "remotes/OrderClient",
        "setups/OrderSetup",
        "services/OrderService",
    ];

    const FILE_FETCH: &str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n";

    const DRIVER: &str = "const problem = { title: 'The request is not valid.', status: 422, errors: [{ field: 'name', rule: 'NotEmpty' }, { field: 'level', rule: 'Max' }] }\n\
         globalThis[Symbol.for('tachyon.tac.http')] = { start: () => ({ answer: Promise.resolve(JSON.stringify({ error: { code: 'HTTP_STATUS', status: 422, body: problem } })), cancel() {} }) }\n\
         const { default: OrderService } = await import('./OrderService.js')\n\
         const order = new OrderService({})\n\
         order.__tachyonStarted?.()\n\
         await order.send()\n\
         const report = { status: order.status, title: order.title, mistakes: order.mistakes }\n\
         order.__tachyonDispose?.()\n\
         console.log(JSON.stringify(report))\n\
         process.exit(0)\n";

    fn copy(root: &Path, extension: &str) {
        let example = Path::new(env!("CARGO_MANIFEST_DIR")).join(Self::EXAMPLE);
        for relative in Self::FILES {
            let file = format!("{relative}.{extension}");
            let text = std::fs::read_to_string(example.join(&file)).expect("example file");
            write(root, &format!("client/{file}"), &text);
        }
    }

    async fn report(language: WebLanguage, extension: &str, prelude: &str) -> serde_json::Value {
        let root = tempfile::tempdir().expect("project");
        Self::copy(root.path(), extension);
        let services = [(
            String::from("order"),
            String::from("OrderService"),
            language,
            root.path()
                .join(format!("client/services/OrderService.{extension}")),
        )];
        let built = build_all(
            root.path(),
            &services,
            &BuildJobs::new(1),
            crate::BuildProfile::Development,
        )
        .await
        .expect("the error body example builds");
        for (name, bytes) in built.files {
            let path = write(root.path(), &name, "");
            std::fs::write(path, bytes).expect("publish");
        }
        run_driver(root.path(), &format!("{prelude}{}", Self::DRIVER))
    }

    fn expected() -> serde_json::Value {
        serde_json::json!({"status": 422, "title": "The request is not valid.", "mistakes": 2})
    }
}

#[tokio::test]
async fn the_swift_error_body_example_reads_the_problem_into_its_model_in_the_browser() {
    if !crate::native::SwiftWebServices::sdk_installed() {
        eprintln!("skipping: Swift WebAssembly SDK is unavailable");
        return;
    }
    let report =
        HttpErrorExample::report(WebLanguage::Swift, "swift", HttpErrorExample::FILE_FETCH).await;
    assert_eq!(report, HttpErrorExample::expected());
}

#[tokio::test]
async fn the_kotlin_error_body_example_reads_the_problem_into_its_model_in_the_browser() {
    if !kotlin_web_ready() {
        eprintln!("skipping: kotlinc-js, TAC_KOTLIN_WASM_STDLIB or node is unavailable");
        return;
    }
    let report = HttpErrorExample::report(WebLanguage::Kotlin, "kt", "").await;
    assert_eq!(report, HttpErrorExample::expected());
}
