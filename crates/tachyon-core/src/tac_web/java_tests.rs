#![allow(clippy::expect_used)]

use super::{BuildJobs, WebLanguage, build_all};
use std::path::Path;
use std::process::Command;

struct JavaFixtures;

impl JavaFixtures {
    const COUNTER_SERVICE: &str = "import java.util.List;\n\n@Service\n@Lifetime({\"value\":\"prototype\"})\nclass CounterService {\n    int count = 0;\n    String label = \"\";\n    double price = 1.5;\n    final String runtime = \"Java\";\n    private int secret = 7;\n\n    \
         @OnStart\n    void start() {\n        label += \" clicks\";\n    }\n\n    \
         int increment() {\n        count += 1;\n        return count;\n    }\n\n    \
         int doubled() {\n        return count * 2;\n    }\n\n    \
         String add(int by, String why) {\n        count += by;\n        return why + \" \" + count;\n    }\n\n    \
         double scale(double factor) {\n        price *= factor;\n        return price;\n    }\n\n    \
         long total(List<Long> values) {\n        long sum = secret - 7;\n        for (long value : values) {\n            sum += value;\n        }\n        return sum;\n    }\n\n    \
         int fail(String why) throws Exception {\n        throw new Exception(\"counter \" + why);\n    }\n}\n";

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
         const total = one.total([1, 2, 2147483648])\n\
         let failed = '', readOnly = ''\n\
         try { one.fail('broke') } catch (error) { failed = error.message }\n\
         try { one.runtime = 'Kotlin' } catch (error) { readOnly = error.message }\n\
         let bigStart = '', bigCall = '', largeStart = ''\n\
         try { new CounterService({ label: 'x'.repeat(1100000) }) } catch (error) { bigStart = error.message }\n\
         try { new CounterService({ label: 'x'.repeat(921600) }).__tachyonDispose() } catch (error) { largeStart = error.message }\n\
         try { one.add(new Event('click'), 1, 'x'.repeat(70000)) } catch (error) { bigCall = error.message }\n\
         one.__tachyonDispose()\n\
         let stale = false\n\
         try { one.count } catch { stale = true }\n\
         console.log(JSON.stringify({ doubled, after, other: two.count, label: two.label, added, scaled, total, runtime: two.runtime, failed, readOnly, stale, keys: Object.keys(two).sort(), bigStart, bigCall, largeStart }))\n\
         process.exit(0)\n";
}

fn java_web_ready() -> bool {
    let runs = |program: &str, flag: &str| {
        Command::new(program)
            .arg(flag)
            .output()
            .is_ok_and(|output| output.status.success())
    };
    crate::native::JavaWebServices::teavm_installed()
        && runs("javac", "-version")
        && runs("node", "--version")
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
    let source = root.join("client/services/CounterService.java");
    std::fs::create_dir_all(source.parent().expect("parent")).expect("folder");
    std::fs::write(&source, JavaFixtures::COUNTER_SERVICE).expect("service");
    let services = [(
        String::from("counter"),
        String::from("CounterService"),
        WebLanguage::Java,
        source,
    )];
    let built = build_all(root, &services, &BuildJobs::new(1), profile)
        .await
        .expect("the Java service builds");
    for (name, bytes) in built.files {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        std::fs::write(path, bytes).expect("publish");
    }
    built.notes
}

#[tokio::test]
async fn a_java_service_runs_in_the_page_as_webassembly() {
    if !java_web_ready() {
        eprintln!("skipping: TeaVM (TAC_TEAVM_HOME), javac or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let notes = build_counter(root.path(), crate::BuildProfile::Development).await;
    assert!(notes.is_empty(), "{notes:?}");
    assert!(root.path().join("java/services.wasm").is_file());
    assert!(root.path().join("java/runtime.js").is_file());
    assert!(root.path().join("tachyon-java.js").is_file());

    let counter = run_driver(root.path(), JavaFixtures::COUNTER_DRIVER);
    assert_eq!(
        counter,
        serde_json::json!({
            "doubled": 4,
            "after": 41,
            "other": 1,
            "label": " clicks",
            "added": "now 50",
            "scaled": 2.5,
            "total": 2_147_483_651_i64,
            "runtime": "Java",
            "failed": "Web service counter: counter broke",
            "readOnly": "Web service counter: Companion field is read-only: runtime",
            "stale": true,
            "keys": ["count", "label", "price", "runtime"],
            "bigStart": "Web service counter: The request is 1100115 bytes. The limit is 1048576 bytes. Pass less data to the service.",
            "bigCall": "Web service counter: The request to add is 70121 bytes. The limit is 65536 bytes. Pass less data to the service.",
            "largeStart": "",
        })
    );
}

#[tokio::test]
async fn a_java_release_build_is_made_smaller_by_binaryen() {
    if !java_web_ready() || !crate::wasm_optimizer::WasmOptimizer::is_installed() {
        eprintln!("skipping: TeaVM, javac, node or wasm-opt is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let development = build_counter(root.path(), crate::BuildProfile::Development).await;
    assert!(development.is_empty(), "{development:?}");
    let unoptimized = std::fs::metadata(root.path().join("java/services.wasm"))
        .expect("development binary")
        .len();
    let notes = build_counter(root.path(), crate::BuildProfile::Release).await;
    assert_eq!(
        notes,
        ["Java: TeaVM WebAssembly release build — optimized by Binaryen"]
    );
    let optimized = std::fs::metadata(root.path().join("java/services.wasm"))
        .expect("release binary")
        .len();
    assert!(optimized < unoptimized, "{optimized} < {unoptimized}");
    let counter = run_driver(root.path(), JavaFixtures::COUNTER_DRIVER);
    assert_eq!(counter["after"], 41);
}

#[tokio::test]
async fn a_java_service_names_the_annotation_it_cannot_use_yet() {
    let root = tempfile::tempdir().expect("project");
    let source = root.path().join("client/services/NoteService.java");
    std::fs::create_dir_all(source.parent().expect("parent")).expect("folder");
    std::fs::write(
        &source,
        "@Service\nclass NoteService {\n    String text = \"\";\n\n    @Cached\n    String read() {\n        return text;\n    }\n}\n",
    )
    .expect("service");
    let services = [(
        String::from("note"),
        String::from("NoteService"),
        WebLanguage::Java,
        source,
    )];
    let Err(failure) = build_all(
        root.path(),
        &services,
        &BuildJobs::new(1),
        crate::BuildProfile::Development,
    )
    .await
    else {
        panic!("an unsupported annotation is refused");
    };
    let text = failure.to_string();
    assert!(text.contains("TY1405"), "{text}");
    assert!(
        text.contains("@Cached does not work in a Java web service yet"),
        "{text}"
    );
}
