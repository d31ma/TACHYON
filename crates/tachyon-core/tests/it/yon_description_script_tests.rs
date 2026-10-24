#![allow(clippy::expect_used)]

use std::process::{Command, Stdio};
use tachyon_contracts::{HandlerRequest, HttpMethod};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
};

use crate::support::toolchain::Toolchain;

struct DescribedScript {
    extension: &'static str,
    runtime: &'static str,
    files: [(&'static str, &'static str); 4],
}

impl DescribedScript {
    const DESCRIPTION: &'static str = "Counts the attempts of every order";

    const JAVASCRIPT: Self = Self {
        extension: "js",
        runtime: "node",
        files: [
            (
                "server/setups/OrdersSetup.js",
                "@Setup\nexport default class OrdersSetup {\n  /** @returns {object} */\n  static configure() { return {}; }\n}\n",
            ),
            (
                "server/services/OrdersService.js",
                "@Service\n@Description(\"Counts the attempts of every order\")\nexport default class OrdersService {\n  /** @type {BehaviorContext} */\n  behaviors;\n  _attempts = 0;\n\n  @OnStart\n  async open() {\n    const signal = new AbortController().signal;\n    this.behaviors = await Behaviors.create(OrdersSetup, { partition: 'orders', signal });\n  }\n\n  /**\n   * @param {string} id\n   * @returns {Promise<{ id: string, attempts: number }>}\n   */\n  @Retry({\"maxRetries\":3,\"delayMs\":10,\"multiplier\":2,\"maxDelayMs\":200,\"jitterMs\":5})\n  async submit(id) {\n    this._attempts += 1;\n    if (this._attempts < 3) throw new Error('the payment gateway is unavailable');\n    return { id, attempts: this._attempts };\n  }\n}\n",
            ),
            (
                "server/models/OrderReport.ts",
                "export interface OrderReport {\n  submitted: unknown\n  again: unknown\n}\n",
            ),
            (
                "server/controllers/OrdersController.js",
                "/** @import { OrderReport } from '/models/OrderReport.ts' */\n@RequestRoute(\"/orders\")\n@Public\nclass OrdersController {\n  /** @type {OrdersService} */\n  @Needs\n  ordersService;\n\n  /** @returns {Promise<OrderReport>} */\n  @GetRoute\n  async getOrders() {\n    const submitted = await this.ordersService.submit('A-1');\n    return { submitted, again: await this.ordersService.submit('A-2') };\n  }\n}\n",
            ),
        ],
    };

    const TYPESCRIPT: Self = Self {
        extension: "ts",
        runtime: "bun",
        files: [
            (
                "server/setups/OrdersSetup.ts",
                "@Setup\nexport default class OrdersSetup {\n  static configure() { return {}; }\n}\n",
            ),
            (
                "server/services/OrdersService.ts",
                "@Service\n@Description(\"Counts the attempts of every order\")\nexport default class OrdersService {\n  behaviors!: BehaviorContext;\n  private attempts = 0;\n\n  @OnStart\n  async open() {\n    const signal = new AbortController().signal;\n    this.behaviors = await Behaviors.create(OrdersSetup, { partition: 'orders', signal });\n  }\n\n  @Retry({\"maxRetries\":3,\"delayMs\":10,\"multiplier\":2,\"maxDelayMs\":200,\"jitterMs\":5})\n  async submit(id: string) {\n    this.attempts += 1;\n    if (this.attempts < 3) throw new Error('the payment gateway is unavailable');\n    return { id, attempts: this.attempts };\n  }\n}\n",
            ),
            (
                "server/models/OrderReport.ts",
                "export interface OrderReport {\n  submitted: unknown\n  again: unknown\n}\n",
            ),
            (
                "server/controllers/OrdersController.ts",
                "import type { OrderReport } from '/models/OrderReport.ts'\n\n@RequestRoute(\"/orders\")\n@Public\nclass OrdersController {\n  @Needs\n  readonly ordersService!: OrdersService;\n\n  @GetRoute\n  async getOrders(): Promise<OrderReport> {\n    const submitted = await this.ordersService.submit('A-1');\n    return { submitted, again: await this.ordersService.submit('A-2') };\n  }\n}\n",
            ),
        ],
    };

    fn available(&self) -> bool {
        let found = Command::new(self.runtime)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success());
        if !Toolchain::checked(self.runtime, found) {
            eprintln!("skipping: {} is unavailable", self.runtime);
            return false;
        }
        true
    }

    fn entry(&self) -> String {
        format!("server/controllers/OrdersController.{}", self.extension)
    }

    fn project(&self, description: &str) -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        for (relative, source) in self.files {
            let target = root.path().join(relative);
            std::fs::create_dir_all(target.parent().expect("parent")).expect("folders");
            let source = source.replace(Self::DESCRIPTION, description);
            std::fs::write(target, source).expect("source");
        }
        root
    }

    fn refuses(&self, description: &str) {
        let root = self.project(description);
        let failure = HandlerSource::discover(root.path(), self.entry())
            .expect_err("a malformed description is refused")
            .to_string();
        assert!(
            failure.contains("Description requires one quoted line"),
            "{} {description:?}: {failure}",
            self.extension
        );
    }

    async fn verify(&self) {
        if !self.available() {
            return;
        }
        let root = self.project(Self::DESCRIPTION);
        let source = HandlerSource::discover(root.path(), self.entry())
            .unwrap_or_else(|failure| panic!("{} discovery: {failure}", self.extension));
        let mut options = HandlerSupervisorOptions::default();
        options.runtimes.javascript = self.runtime.into();
        let supervisor = HandlerSupervisor::new(options).expect("supervisor");
        let response = supervisor
            .invoke(
                &source,
                &HandlerRequest::route("described", "/orders", HttpMethod::Get),
                &HandlerCancellation::default(),
            )
            .await
            .unwrap_or_else(|error| panic!("{} invocation: {error:?}", self.extension));
        assert!(
            response.error.is_none(),
            "{}: {:?}",
            self.extension,
            response.error
        );
        assert_eq!(response.status, 200, "{}", self.extension);
        let body: serde_json::Value =
            serde_json::from_str(&response.body.expect("body").data).expect("JSON body");
        assert_eq!(
            body,
            serde_json::json!({
                "submitted": {"id": "A-1", "attempts": 3},
                "again": {"id": "A-2", "attempts": 4},
            }),
            "{}: {body}",
            self.extension
        );
        for malformed in ["Counts \\n orders", "", "Counts 'quoted' orders"] {
            self.refuses(malformed);
        }
    }
}

#[tokio::test]
async fn a_described_javascript_service_keeps_its_behavior_and_refuses_a_malformed_description() {
    DescribedScript::JAVASCRIPT.verify().await;
}

#[tokio::test]
async fn a_described_typescript_service_keeps_its_behavior_and_refuses_a_malformed_description() {
    DescribedScript::TYPESCRIPT.verify().await;
}
