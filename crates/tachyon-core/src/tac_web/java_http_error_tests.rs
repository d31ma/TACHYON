#![allow(clippy::expect_used)]

use super::java_tests::{java_web_ready, publish, run_driver, write};
use std::path::Path;

struct JavaHttpErrors;

impl JavaHttpErrors {
    const FILES: [(&str, &str); 4] = [
        (
            "client/models/ClaimConflict.java",
            "record ClaimConflict(String reason, String holder) {}\n",
        ),
        (
            "client/remotes/ClaimClient.java",
            "@HTTPClient(\"/api\")\ninterface ClaimClient {\n    @PutRequest(\"/claims/{id}\")\n    Object claim(@PathVariable String id, @RequestBody Object claim);\n}\n",
        ),
        (
            "client/setups/ClaimSetup.java",
            "@Setup\nclass ClaimSetup {\n    static Behaviors.Configuration configure(Behaviors.Execution execution) {\n        return new Behaviors.Configuration();\n    }\n}\n",
        ),
        (
            "client/services/ClaimService.java",
            "@Service\nclass ClaimService {\n    @Needs\n    private final ClaimClient claimClient;\n    String code = \"\";\n    int status = 0;\n    String reason = \"\";\n    String holder = \"\";\n\n    \
             void claim(String id) {\n        try {\n            claimClient.claim(id, java.util.Map.of(\"item\", \"saw\"));\n        } catch (Behaviors.Failure failure) {\n            \
             ClaimConflict conflict = Behaviors.errorBody(failure, TacModels::readClaimConflict).orElse(new ClaimConflict(\"none\", \"none\"));\n            \
             code = failure.code;\n            status = Behaviors.errorStatus(failure);\n            reason = conflict.reason();\n            holder = conflict.holder();\n        }\n    }\n}\n",
        ),
    ];

    const DRIVER: &str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n\
         const sent = []\n\
         const answers = [\n\
           { error: { code: 'HTTP_STATUS', status: 409, body: { reason: 'taken', holder: 'south' } } },\n\
           { error: { code: 'HTTP_STATUS', status: 503 } },\n\
           { error: { code: 'HTTP_NETWORK' } },\n\
         ]\n\
         globalThis[Symbol.for('tachyon.tac.http')] = { start: (text) => { sent.push(JSON.parse(text)); return { answer: Promise.resolve(JSON.stringify(answers.shift())), cancel() {} } } }\n\
         const { default: ClaimService } = await import('./ClaimService.js')\n\
         const claims = new ClaimService({})\n\
         claims.__tachyonStarted()\n\
         const read = () => ({ code: claims.code, status: claims.status, reason: claims.reason, holder: claims.holder })\n\
         await claims.claim('a-7')\n\
         const conflict = read()\n\
         await claims.claim('b-8')\n\
         const unavailable = read()\n\
         await claims.claim('c-9')\n\
         const network = read()\n\
         claims.__tachyonDispose()\n\
         console.log(JSON.stringify({ conflict, unavailable, network, sent: sent.map((request) => [request.method, request.path, request.body]) }))\n\
         process.exit(0)\n";

    const EXAMPLE: &str = "../../website/content/docs/calling-the-server/http-error-bodies/client";
    const EXAMPLE_FILES: [&str; 6] = [
        "models/OrderInput.java",
        "models/Problem.java",
        "models/ProblemEntry.java",
        "remotes/OrderClient.java",
        "setups/OrderSetup.java",
        "services/OrderService.java",
    ];

    const EXAMPLE_DRIVER: &str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n\
         const problem = { title: 'The request is not valid.', status: 422, errors: [{ field: 'name', rule: 'NotEmpty' }, { field: 'level', rule: 'Max' }] }\n\
         globalThis[Symbol.for('tachyon.tac.http')] = { start: () => ({ answer: Promise.resolve(JSON.stringify({ error: { code: 'HTTP_STATUS', status: 422, body: problem } })), cancel() {} }) }\n\
         const { default: OrderService } = await import('./OrderService.js')\n\
         const order = new OrderService({})\n\
         order.__tachyonStarted()\n\
         await order.send()\n\
         const report = { status: order.status, title: order.title, mistakes: order.mistakes }\n\
         order.__tachyonDispose()\n\
         console.log(JSON.stringify(report))\n\
         process.exit(0)\n";

    fn copy_example(root: &Path) {
        let example = Path::new(env!("CARGO_MANIFEST_DIR")).join(Self::EXAMPLE);
        for relative in Self::EXAMPLE_FILES {
            let text = std::fs::read_to_string(example.join(relative)).expect("example file");
            write(root, &format!("client/{relative}"), &text);
        }
    }
}

#[tokio::test]
async fn a_java_web_service_reads_an_http_error_body_into_a_model() {
    if !java_web_ready() {
        eprintln!("skipping: TeaVM (TAC_TEAVM_HOME), javac or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    for (relative, text) in JavaHttpErrors::FILES {
        write(root.path(), relative, text);
    }
    publish(root.path(), "claims", "ClaimService").await;
    let report = run_driver(root.path(), JavaHttpErrors::DRIVER);
    assert_eq!(
        report["conflict"],
        serde_json::json!({"code": "HTTP_STATUS", "status": 409, "reason": "taken", "holder": "south"})
    );
    assert_eq!(
        report["unavailable"],
        serde_json::json!({"code": "HTTP_STATUS", "status": 503, "reason": "none", "holder": "none"})
    );
    assert_eq!(
        report["network"],
        serde_json::json!({"code": "HTTP_NETWORK", "status": 0, "reason": "none", "holder": "none"})
    );
    assert_eq!(
        report["sent"][0],
        serde_json::json!(["PUT", "/api/claims/a-7", {"item": "saw"}])
    );
}

#[tokio::test]
async fn the_java_error_body_example_reads_the_problem_into_its_model() {
    if !java_web_ready() {
        eprintln!("skipping: TeaVM (TAC_TEAVM_HOME), javac or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    JavaHttpErrors::copy_example(root.path());
    publish(root.path(), "order", "OrderService").await;
    let report = run_driver(root.path(), JavaHttpErrors::EXAMPLE_DRIVER);
    assert_eq!(
        report,
        serde_json::json!({"status": 422, "title": "The request is not valid.", "mistakes": 2})
    );
}
