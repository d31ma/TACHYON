use crate::node_driver::NodeDriver;
use serde_json::{Value, json};

struct RandomIdDriver;

impl RandomIdDriver {
    const DRIVER: &'static str = r"import { readFile } from 'node:fs/promises'
const source = await readFile('./tac-random-id.js', 'utf8')
const load = () => (0, eval)(`${source}\nTacRandomId`)
const shape = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/
const native = load()
const realRandomUUID = globalThis.crypto.randomUUID
Object.defineProperty(globalThis.crypto, 'randomUUID', { configurable: true, value: () => 'native-id' })
const delegated = native.next()
Object.defineProperty(globalThis.crypto, 'randomUUID', { configurable: true, value: undefined })
const fallback = load()
const made = Array.from({ length: 1000 }, () => fallback.next())
Object.defineProperty(globalThis.crypto, 'randomUUID', { configurable: true, value: realRandomUUID })
console.log(JSON.stringify({
  delegated,
  shaped: made.every((id) => shape.test(id)),
  distinct: new Set(made).size,
}))
process.exit(0)
";

    fn run() -> Value {
        let root = tempfile::tempdir().expect("root");
        for (name, source) in [
            ("tac-random-id.js", include_str!("tac-random-id.js")),
            ("drive.mjs", Self::DRIVER),
        ] {
            std::fs::write(root.path().join(name), source).expect("write driver file");
        }
        let mut command = NodeDriver::command();
        command.args(["drive.mjs"]).current_dir(root.path());
        let output = NodeDriver::output(&mut command).expect("node drive.mjs");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "{stdout}{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_str(stdout.lines().last().unwrap_or_default()).expect("report")
    }
}

#[test]
fn random_ids_use_the_browser_uuid_and_fall_back_without_a_secure_context() {
    assert_eq!(
        RandomIdDriver::run(),
        json!({ "delegated": "native-id", "shaped": true, "distinct": 1000 })
    );
}

#[test]
fn shipped_page_scripts_make_ids_only_through_the_fallback_helper() {
    for (name, source) in [
        ("tac-client.js", include_str!("tac-client.js")),
        ("tac-files.js", include_str!("tac-files.js")),
        (
            "tac-dotnet-http.js",
            include_str!("../native/browser/tac-dotnet-http.js"),
        ),
    ] {
        assert!(
            !source.contains("crypto.randomUUID"),
            "{name} calls crypto.randomUUID directly"
        );
        assert!(source.contains("TacRandomId.next()"), "{name}");
    }
    assert!(super::ClientViewRenderer::RUNTIME.starts_with("class TacRandomId {"));
}
