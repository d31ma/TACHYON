#![allow(clippy::expect_used)]

use super::kotlin_tests::kotlin_web_ready;
use super::{BuildJobs, WebLanguage, build_all};
use crate::node_driver::NodeDriver;
use serde_json::json;
use std::path::Path;

struct KotlinRetry;

impl KotlinRetry {
    const FILES: [(&str, &str); 2] = [
        (
            "client/setups/OrdersSetup.kt",
            "@Setup\nclass OrdersSetup {\n    companion object {\n        \
             fun configure(execution: Behaviors.Execution): Behaviors.Configuration =\n            \
             Behaviors.Configuration(caches = mapOf(), authorize = Behaviors.Authorization { _, _ -> true })\n    }\n}\n",
        ),
        (
            "client/services/OrdersService.kt",
            "@Service\nclass OrdersService {\n    \
             private lateinit var behaviors: Behaviors.Context\n    \
             var attempts: Int = 0\n    \
             var brokenAttempts: Int = 0\n\n    \
             @OnStart\n    fun open() {\n        \
             behaviors = Behaviors.create(\"OrdersSetup\", Behaviors.Options(\"orders\", Behaviors.Signal()))\n    }\n\n    \
             @Retry({\"maxRetries\":3,\"delayMs\":10,\"multiplier\":2,\"maxDelayMs\":200,\"jitterMs\":5})\n    \
             fun submit(id: String): Any? {\n        \
             attempts++\n        \
             if (attempts < 3) throw IllegalStateException(\"the checkout API is unavailable\")\n        \
             return mapOf(\"id\" to id, \"attempts\" to attempts)\n    }\n\n    \
             @Retry({\"maxRetries\":2,\"delayMs\":1,\"multiplier\":1,\"maxDelayMs\":5,\"jitterMs\":0})\n    \
             fun broken(): Any? {\n        \
             brokenAttempts++\n        \
             throw IllegalStateException(\"still down\")\n    }\n\n    \
             fun plain(): Any? {\n        \
             attempts++\n        \
             throw IllegalStateException(\"not retried\")\n    }\n}\n",
        ),
    ];

    const DRIVER: &str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n\
         console.info = () => {}\n\
         const { default: OrdersService } = await import('./OrdersService.js')\n\
         const outcome = (run) => { try { return { value: run() } } catch (error) { return { error: String(error?.message ?? error) } } }\n\
         const orders = new OrdersService({})\n\
         orders.__tachyonStarted()\n\
         const submitted = outcome(() => orders.submit('A1'))\n\
         const attemptsAfterSubmit = orders.attempts\n\
         const broken = outcome(() => orders.broken())\n\
         const plain = outcome(() => orders.plain())\n\
         console.log(JSON.stringify({ submitted, attemptsAfterSubmit, broken, brokenAttempts: orders.brokenAttempts, plain, attemptsAfterPlain: orders.attempts }))\n\
         process.exit(0)\n";

    fn write(root: &Path, relative: &str, text: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        std::fs::write(&path, text).expect("file");
    }

    async fn publish(root: &Path) {
        for (relative, text) in Self::FILES {
            Self::write(root, relative, text);
        }
        let services = vec![(
            String::from("orders"),
            String::from("OrdersService"),
            WebLanguage::Kotlin,
            root.join("client/services/OrdersService.kt"),
        )];
        let built = build_all(
            root,
            &services,
            &BuildJobs::new(1),
            crate::BuildProfile::Development,
        )
        .await
        .unwrap_or_else(|failure| panic!("{failure}"));
        for (name, bytes) in built.files {
            let path = root.join(name);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
            std::fs::write(path, bytes).expect("publish");
        }
    }

    fn drive(root: &Path) -> serde_json::Value {
        std::fs::write(root.join("drive.mjs"), Self::DRIVER).expect("driver");
        let output = NodeDriver::output(NodeDriver::command().arg("drive.mjs").current_dir(root))
            .expect("node");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "{stdout}{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_str(stdout.trim()).expect("report")
    }
}

#[tokio::test]
async fn a_kotlin_retry_method_runs_a_failed_call_again_on_the_page() {
    if !kotlin_web_ready() {
        eprintln!("skipping: kotlinc-js, TAC_KOTLIN_WASM_STDLIB or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    KotlinRetry::publish(root.path()).await;
    let report = KotlinRetry::drive(root.path());

    assert_eq!(
        report["submitted"],
        json!({"value": {"id": "A1", "attempts": 3}}),
        "{report}"
    );
    assert_eq!(report["attemptsAfterSubmit"], 3, "{report}");
    assert!(
        report["broken"]["error"]
            .as_str()
            .is_some_and(|message| message.contains("still down")),
        "{report}"
    );
    assert_eq!(report["brokenAttempts"], 3, "{report}");
    assert!(report["plain"]["error"].is_string(), "{report}");
    assert_eq!(report["attemptsAfterPlain"], 4, "{report}");
}
