#![allow(clippy::expect_used)]

use super::{BuildJobs, WebLanguage, build_all};
use std::path::{Path, PathBuf};
use std::process::Command;

struct SwiftFixtures;

impl SwiftFixtures {
    const COUNTER_SERVICE: &str = "@Service\n@Lifetime({\"value\":\"prototype\"})\nfinal class CounterService {\n    var count: Int = 0\n    var label: String = \"\"\n    var price: Double = 1.5\n    let runtime = \"Swift\"\n\n    \
         struct CounterFailure: Error, CustomStringConvertible {\n        let description: String\n    }\n\n    \
         @OnStart\n    func start() {\n        label += \" clicks\"\n    }\n\n    \
         func increment() -> Int {\n        count += 1\n        return count\n    }\n\n    \
         func doubled() -> Int { count * 2 }\n\n    \
         func add(_ by: Int, why: String) -> String {\n        count += by\n        return why + \" \" + String(count)\n    }\n\n    \
         func scale(by factor: Double) -> Double {\n        price *= factor\n        return price\n    }\n\n    \
         func fail(_ why: String) throws -> Int {\n        throw CounterFailure(description: \"counter \" + why)\n    }\n}\n";

    const COUNTER_DRIVER: &str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n\
         const { default: CounterService } = await import('./CounterService.js')\n\
         const one = new CounterService({}), two = new CounterService({})\n\
         one.__tachyonStarted(); two.__tachyonStarted()\n\
         one.increment(); one.increment(); two.increment()\n\
         const doubled = one.doubled()\n\
         one.count = 40\n\
         const after = one.increment()\n\
         const added = one.add(new Event('click'), 9, 'now')\n\
         one.price = 2\n\
         const scaled = one.scale(1.25)\n\
         let failed = ''\n\
         try { one.fail('broke') } catch (error) { failed = error.message }\n\
         one.__tachyonDispose()\n\
         let stale = false\n\
         try { one.count } catch { stale = true }\n\
         console.log(JSON.stringify({ doubled, after, other: two.count, label: two.label, added, scaled, runtime: two.runtime, failed, stale, keys: Object.keys(two).sort() }))\n\
         process.exit(0)\n";
}

pub(crate) fn swift_web_ready() -> bool {
    let sdk = std::env::var_os("TAC_SWIFT_WASM_SDK").map(PathBuf::from);
    let has_sdk = sdk.is_some_and(|path| path.join("WASI.sdk").is_dir());
    let compiler = std::env::var_os("TAC_SWIFT_WASM_COMPILER").unwrap_or_else(|| "swiftc".into());
    let runs = |program: &std::ffi::OsStr, flag: &str| {
        Command::new(program)
            .arg(flag)
            .output()
            .is_ok_and(|output| output.status.success())
    };
    has_sdk && runs(&compiler, "--version") && runs("node".as_ref(), "--version")
}

fn run_driver(root: &Path, script: &str) -> serde_json::Value {
    std::fs::write(root.join("drive.mjs"), script).expect("driver");
    let output = Command::new("node")
        .arg("drive.mjs")
        .current_dir(root)
        .output()
        .expect("node");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{stdout}{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_str(stdout.trim()).expect("report")
}

async fn build_counter(root: &Path, profile: crate::BuildProfile) -> Vec<String> {
    let source = root.join("client/services/CounterService.swift");
    std::fs::create_dir_all(source.parent().expect("parent")).expect("folder");
    std::fs::write(&source, SwiftFixtures::COUNTER_SERVICE).expect("service");
    let services = [(
        String::from("counter"),
        String::from("CounterService"),
        WebLanguage::Swift,
        source,
    )];
    let built = build_all(root, &services, &BuildJobs::new(1), profile)
        .await
        .expect("the Swift service builds");
    for (name, bytes) in built.files {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        std::fs::write(path, bytes).expect("publish");
    }
    built.notes
}

#[tokio::test]
async fn a_swift_service_runs_in_the_page_as_webassembly() {
    if !swift_web_ready() {
        eprintln!("skipping: the Swift SDK for WebAssembly, swiftc or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let notes = build_counter(root.path(), crate::BuildProfile::Development).await;
    assert!(notes.is_empty(), "{notes:?}");
    assert!(root.path().join("swift/services.wasm").is_file());
    assert!(root.path().join("tachyon-swift.js").is_file());

    let counter = run_driver(root.path(), SwiftFixtures::COUNTER_DRIVER);
    assert_eq!(
        counter,
        serde_json::json!({
            "doubled": 4,
            "after": 41,
            "other": 1,
            "label": " clicks",
            "added": "now 50",
            "scaled": 2.5,
            "runtime": "Swift",
            "failed": "Web service counter: counter broke",
            "stale": true,
            "keys": ["count", "label", "price", "runtime"],
        })
    );
}

#[tokio::test]
async fn a_swift_release_build_is_made_smaller_by_binaryen() {
    if !swift_web_ready() || !crate::wasm_optimizer::WasmOptimizer::is_installed() {
        eprintln!(
            "skipping: the Swift SDK for WebAssembly, swiftc, node or wasm-opt is unavailable"
        );
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let notes = build_counter(root.path(), crate::BuildProfile::Release).await;
    assert_eq!(
        notes,
        ["Swift: WebAssembly release build — optimized by Binaryen"]
    );
    let counter = run_driver(root.path(), SwiftFixtures::COUNTER_DRIVER);
    assert_eq!(counter["after"], 41);
}
