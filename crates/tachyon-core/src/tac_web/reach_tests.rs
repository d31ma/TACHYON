#![allow(clippy::expect_used)]

use super::WebLanguage;
use std::path::Path;
use std::process::Command;

struct Reach;

impl Reach {
    const FILE_FETCH: &str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n";

    const DRIVER: &str = "const lines = []\n\
         for (const level of ['log', 'info', 'warn', 'error']) console[level] = (...values) => lines.push(values.join(' '))\n\
         const { default: ReachService } = await import('./ReachService.js')\n\
         const reach = new ReachService({})\n\
         const report = { before: Date.now() }\n\
         for (const name of process.argv.slice(2)) {\n\
         \x20 try { report[name] = { value: await reach[name]() } } catch (error) { report[name] = { error: String(error?.message ?? error) } }\n\
         }\n\
         report.after = Date.now()\n\
         report.lines = lines.filter((line) => line.includes('reach-'))\n\
         process.stdout.write(`${JSON.stringify(report)}\\n`)\n\
         process.exit(0)\n";

    const RUST: &str = "#[Service()]\n#[derive(Default)]\nstruct ReachService {}\n\n\
         impl ReachService {\n    \
         fn stamp(&self) -> f64 {\n        \
         let elapsed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();\n        \
         elapsed.as_secs_f64() * 1000.0\n    }\n\n    \
         fn say(&self) -> bool {\n        println!(\"reach-log\");\n        eprintln!(\"reach-error\");\n        true\n    }\n\n    \
         fn pause(&self) -> bool {\n        std::thread::sleep(std::time::Duration::from_millis(5));\n        true\n    }\n}\n";

    const GO: &str = "import (\n\tcryptorand \"crypto/rand\"\n\t\"fmt\"\n\t\"math/rand/v2\"\n\t\"time\"\n)\n\n\
         // @Service()\ntype ReachService struct{}\n\n\
         func (reach *ReachService) Stamp() float64 {\n\treturn float64(time.Now().UnixMilli())\n}\n\n\
         func (reach *ReachService) Chance() float64 {\n\treturn rand.Float64()\n}\n\n\
         func (reach *ReachService) Secure() int {\n\tbuffer := make([]byte, 8)\n\tcount, _ := cryptorand.Read(buffer)\n\treturn count\n}\n\n\
         func (reach *ReachService) Say() bool {\n\tfmt.Println(\"reach-log\")\n\treturn true\n}\n\n\
         func (reach *ReachService) Pause() bool {\n\ttime.Sleep(5 * time.Millisecond)\n\treturn true\n}\n";

    const CSHARP: &str = "[Service()]\npublic class ReachService\n{\n    \
         public double Stamp() => System.DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();\n\n    \
         public double Chance() => System.Random.Shared.NextDouble();\n\n    \
         public int Secure() => System.Security.Cryptography.RandomNumberGenerator.GetBytes(8).Length;\n\n    \
         public bool Say()\n    {\n        System.Console.WriteLine(\"reach-log\");\n        return true;\n    }\n\n    \
         public bool Pause()\n    {\n        System.Threading.Thread.Sleep(5);\n        return true;\n    }\n}\n";

    const DART: &str = "import 'dart:math';\n\n@Service()\nclass ReachService {\n  \
         double stamp() => DateTime.now().millisecondsSinceEpoch.toDouble();\n\n  \
         double chance() => Random().nextDouble();\n\n  \
         int secure() => Random.secure().nextInt(256);\n\n  \
         bool say() {\n    print('reach-log');\n    return true;\n  }\n\n  \
         Future<bool> pause() async {\n    await Future<void>.delayed(const Duration(milliseconds: 5));\n    return true;\n  }\n}\n";

    const RUBY: &str = "#[Service]\nclass ReachService\n  \
         def stamp\n    Time.now.to_f * 1000\n  end\n\n  \
         def chance\n    rand\n  end\n\n  \
         def say\n    puts \"reach-log\"\n    true\n  end\n\n  \
         def pause\n    sleep 0.005\n    true\n  end\nend\n";

    const RUST_UTILITY: &str = "pub struct TextUtils;\n\nimpl TextUtils {\n    \
         pub fn shout(text: &str) -> String {\n        format!(\"{}!\", text.to_uppercase())\n    }\n}\n";

    const RUST_SHOUT: &str = "#[Service()]\n#[derive(Default)]\nstruct ReachService {}\n\n\
         impl ReachService {\n    fn shout(&self) -> String {\n        TextUtils::shout(\"reach\")\n    }\n}\n";

    const GO_UTILITY: &str = "import \"strings\"\n\ntype TextUtils struct{}\n\n\
         func (TextUtils) Shout(text string) string {\n\treturn strings.ToUpper(text) + \"!\"\n}\n";

    const GO_SHOUT: &str = "// @Service()\ntype ReachService struct{}\n\n\
         func (reach *ReachService) Shout() string {\n\treturn TextUtils{}.Shout(\"reach\")\n}\n";

    const CSHARP_UTILITY: &str = "public static class TextUtils\n{\n    \
         public static string Shout(string text) => text.ToUpper() + \"!\";\n}\n";

    const CSHARP_SHOUT: &str = "[Service()]\npublic class ReachService\n{\n    \
         public string Shout() => TextUtils.Shout(\"reach\");\n}\n";

    const RUBY_UTILITY: &str =
        "class TextUtils\n  def self.shout(text)\n    \"#{text.upcase}!\"\n  end\nend\n";

    const RUBY_SHOUT: &str =
        "#[Service]\nclass ReachService\n  def shout\n    TextUtils.shout(\"reach\")\n  end\nend\n";

    const SHOUTED: &str = "REACH!";

    fn available(program: &str, argument: &str) -> bool {
        Command::new(program)
            .arg(argument)
            .output()
            .is_ok_and(|output| output.status.success())
    }

    fn rust_web_target() -> bool {
        let Ok(output) = Command::new("rustc")
            .args([
                "--print",
                "target-libdir",
                "--target",
                crate::native::RustCompanion::WEB_TARGET,
            ])
            .output()
        else {
            return false;
        };
        Path::new(String::from_utf8_lossy(&output.stdout).trim()).is_dir()
    }

    fn write(root: &Path, relative: &str, text: &str) -> std::path::PathBuf {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(&path, text).expect("write");
        path
    }

    async fn publish(root: &Path, language: WebLanguage, source: &str, extension: &str) {
        Self::publish_with(
            root,
            language,
            (source, extension),
            crate::BuildProfile::Release,
        )
        .await;
    }

    async fn publish_with(
        root: &Path,
        language: WebLanguage,
        (source, extension): (&str, &str),
        profile: crate::BuildProfile,
    ) {
        let path = Self::write(
            root,
            &format!("client/services/ReachService.{extension}"),
            source,
        );
        let services = [(
            String::from("reach"),
            String::from("ReachService"),
            language,
            path,
        )];
        let jobs = super::BuildJobs::new(1);
        let published = super::build_all(root, &services, &jobs, profile)
            .await
            .expect("build");
        for (name, bytes) in published.files {
            let _ = Self::write_bytes(root, &name, &bytes);
        }
    }

    async fn shout_through_utility(
        language: WebLanguage,
        (service, utility, extension): (&str, &str, &str),
        prefix: &str,
    ) -> serde_json::Value {
        let root = tempfile::tempdir().expect("project");
        let utility_path = format!("client/utils/TextUtils.{extension}");
        let _ = Self::write(root.path(), &utility_path, utility);
        let profile = crate::BuildProfile::Development;
        Self::publish_with(root.path(), language, (service, extension), profile).await;
        Self::run(root.path(), prefix, &["shout"])
    }

    fn write_bytes(root: &Path, relative: &str, bytes: &[u8]) -> std::path::PathBuf {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(&path, bytes).expect("publish");
        path
    }

    async fn publish_dart(root: &Path) {
        let path = Self::write(root, "client/services/ReachService.dart", Self::DART);
        let services = [("ReachService", path.as_path())];
        let instances = std::collections::BTreeSet::from([String::from("reach")]);
        let modules = crate::tac_dart::build_all(root, &services, &instances, crate::BuildProfile::Release)
            .await
            .expect("build");
        for (name, bytes) in modules {
            let _ = Self::write_bytes(root, &name, &bytes);
        }
        Self::write(root, "package.json", "{\"type\":\"module\"}");
    }

    fn run(root: &Path, prefix: &str, methods: &[&str]) -> serde_json::Value {
        std::fs::write(root.join("reach.mjs"), format!("{prefix}{}", Self::DRIVER))
            .expect("driver");
        let output = Command::new("node")
            .arg("reach.mjs")
            .args(methods)
            .current_dir(root)
            .output()
            .expect("node");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "{stdout}{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let last = stdout.lines().last().unwrap_or_default();
        serde_json::from_str(last).expect("report")
    }

    fn assert_clock(report: &serde_json::Value) {
        let stamp = report["stamp"]["value"].as_f64().expect("a clock value");
        let before = report["before"].as_f64().expect("before");
        let after = report["after"].as_f64().expect("after");
        assert!(
            stamp + 1000.0 >= before && stamp <= after + 1000.0,
            "{report}"
        );
    }

    fn assert_chance(report: &serde_json::Value) {
        let chance = report["chance"]["value"].as_f64().expect("a random value");
        assert!((0.0..1.0).contains(&chance), "{report}");
    }

    fn assert_portable(report: &serde_json::Value) {
        Self::assert_clock(report);
        Self::assert_chance(report);
        assert_eq!(report["say"]["value"], true, "{report}");
        assert_eq!(
            report["lines"],
            serde_json::json!(["reach-log"]),
            "{report}"
        );
    }
}

#[tokio::test]
async fn a_rust_web_service_has_no_clock_sleep_or_console_of_its_own() {
    if !Reach::available("node", "--version") || !Reach::rust_web_target() {
        eprintln!("skipping: node or the wasm32 Rust target is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    Reach::publish(root.path(), WebLanguage::Rust, Reach::RUST, "rs").await;
    let said = Reach::run(root.path(), "", &["say"]);
    assert_eq!(said["say"]["value"], true, "{said}");
    assert_eq!(said["lines"], serde_json::json!([]), "{said}");
    let stamped = Reach::run(root.path(), "", &["stamp"]);
    assert!(stamped["stamp"]["error"].is_string(), "{stamped}");
    let paused = Reach::run(root.path(), "", &["pause"]);
    assert!(paused["pause"]["error"].is_string(), "{paused}");
}

#[tokio::test]
async fn a_go_web_service_has_a_clock_random_numbers_a_console_and_sleep() {
    if !Reach::available("node", "--version") || !Reach::available("go", "version") {
        eprintln!("skipping: node or go is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    Reach::publish(root.path(), WebLanguage::Go, Reach::GO, "go").await;
    let methods = ["stamp", "chance", "secure", "say", "pause"];
    let report = Reach::run(root.path(), Reach::FILE_FETCH, &methods);
    Reach::assert_portable(&report);
    assert_eq!(report["secure"]["value"], 8, "{report}");
    assert_eq!(report["pause"]["value"], true, "{report}");
}

#[tokio::test]
async fn a_csharp_web_service_has_a_clock_random_numbers_a_console_and_sleep() {
    let workload = Command::new("dotnet")
        .args(["workload", "list"])
        .output()
        .is_ok_and(|output| String::from_utf8_lossy(&output.stdout).contains("wasm-tools"));
    if !Reach::available("node", "--version") || !workload {
        eprintln!("skipping: node or the .NET wasm-tools workload is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = (Reach::CSHARP, "cs");
    Reach::publish_with(
        root.path(),
        WebLanguage::CSharp,
        source,
        crate::BuildProfile::Development,
    )
    .await;
    let report = Reach::run(
        root.path(),
        "",
        &["stamp", "chance", "secure", "say", "pause"],
    );
    Reach::assert_portable(&report);
    assert_eq!(report["secure"]["value"], 8, "{report}");
    assert_eq!(report["pause"]["value"], true, "{report}");
}

#[tokio::test]
async fn a_dart_service_has_a_clock_random_numbers_a_console_and_timers() {
    if !Reach::available("node", "--version") || !Reach::available("dart", "--version") {
        eprintln!("skipping: node or dart is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    Reach::publish_dart(root.path()).await;
    let report = Reach::run(
        root.path(),
        Reach::FILE_FETCH,
        &["stamp", "chance", "secure", "say", "pause"],
    );
    Reach::assert_portable(&report);
    assert!(report["secure"]["value"].as_f64().is_some(), "{report}");
    assert_eq!(report["pause"]["value"], true, "{report}");
}

#[tokio::test]
async fn a_ruby_web_service_has_a_clock_random_numbers_a_console_and_no_sleep() {
    let unavailable = crate::tac_ruby::RubyRuntimeBuild::unavailable_reason();
    if !Reach::available("node", "--version") || unavailable.is_some() {
        eprintln!("skipping: node or the Ruby runtime is unavailable: {unavailable:?}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    Reach::publish(root.path(), WebLanguage::Ruby, Reach::RUBY, "rb").await;
    let report = Reach::run(
        root.path(),
        Reach::FILE_FETCH,
        &["stamp", "chance", "say", "pause"],
    );
    Reach::assert_portable(&report);
    let refused = report["pause"]["error"].as_str().unwrap_or_default();
    assert!(refused.contains("sleep would freeze the page"), "{report}");
}

#[tokio::test]
async fn a_rust_web_service_calls_a_utility_class_without_an_import() {
    if !Reach::available("node", "--version") || !Reach::rust_web_target() {
        eprintln!("skipping: node or the wasm32 Rust target is unavailable");
        return;
    }
    let sources = (Reach::RUST_SHOUT, Reach::RUST_UTILITY, "rs");
    let report = Reach::shout_through_utility(WebLanguage::Rust, sources, "").await;
    assert_eq!(report["shout"]["value"], Reach::SHOUTED, "{report}");
}

#[tokio::test]
async fn a_go_web_service_calls_a_utility_class_without_an_import() {
    if !Reach::available("node", "--version") || !Reach::available("go", "version") {
        eprintln!("skipping: node or go is unavailable");
        return;
    }
    let sources = (Reach::GO_SHOUT, Reach::GO_UTILITY, "go");
    let report = Reach::shout_through_utility(WebLanguage::Go, sources, Reach::FILE_FETCH).await;
    assert_eq!(report["shout"]["value"], Reach::SHOUTED, "{report}");
}

#[tokio::test]
async fn a_csharp_web_service_calls_a_utility_class_without_an_import() {
    let workload = Command::new("dotnet")
        .args(["workload", "list"])
        .output()
        .is_ok_and(|output| String::from_utf8_lossy(&output.stdout).contains("wasm-tools"));
    if !Reach::available("node", "--version") || !workload {
        eprintln!("skipping: node or the .NET wasm-tools workload is unavailable");
        return;
    }
    let sources = (Reach::CSHARP_SHOUT, Reach::CSHARP_UTILITY, "cs");
    let report = Reach::shout_through_utility(WebLanguage::CSharp, sources, "").await;
    assert_eq!(report["shout"]["value"], Reach::SHOUTED, "{report}");
}

#[tokio::test]
async fn a_ruby_web_service_calls_a_utility_class_without_an_import() {
    let unavailable = crate::tac_ruby::RubyRuntimeBuild::unavailable_reason();
    if !Reach::available("node", "--version") || unavailable.is_some() {
        eprintln!("skipping: node or the Ruby runtime is unavailable: {unavailable:?}");
        return;
    }
    let sources = (Reach::RUBY_SHOUT, Reach::RUBY_UTILITY, "rb");
    let report = Reach::shout_through_utility(WebLanguage::Ruby, sources, Reach::FILE_FETCH).await;
    assert_eq!(report["shout"]["value"], Reach::SHOUTED, "{report}");
}
