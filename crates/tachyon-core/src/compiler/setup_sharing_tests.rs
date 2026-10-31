#![allow(clippy::expect_used)]

use super::setup_sharing::SetupSharing;
use super::{BuildOptions, WebCompiler};
use crate::one_test_per_case_tests::one_test_per_case;
use std::path::Path;

#[test]
fn one_setup_text_moves_out_of_every_service_module() {
    let alpha = SetupModuleFixture::module("AlphaService", SetupModuleFixture::PLAIN_SETUP, "[]");
    let beta = SetupModuleFixture::module("BetaService", SetupModuleFixture::PLAIN_SETUP, "[]");
    let sharing = SetupSharing::plan(&[alpha.as_bytes(), beta.as_bytes()]);
    let (alpha_without, alpha_names) = sharing.without_setups(alpha.as_bytes());
    let (beta_without, beta_names) = sharing.without_setups(beta.as_bytes());
    let exports = String::from_utf8(sharing.exports()).expect("text");
    assert_eq!(alpha_names, vec![String::from("PageSetup")]);
    assert_eq!(beta_names, vec![String::from("PageSetup")]);
    assert_eq!(exports.matches("export class PageSetup {").count(), 1);
    assert!(exports.contains("static configured = 0;"));
    for without in [alpha_without, beta_without] {
        let text = String::from_utf8(without).expect("text");
        assert!(!text.contains("class PageSetup"), "{text}");
        assert!(text.contains("{type:PageSetup,"), "{text}");
    }
}

#[test]
fn a_setup_stays_inline_when_its_modules_cannot_share_it() {
    let reads_module_class = "class PageSetup {\n  static configure() { return { http: { page: new AlphaService() } }; }\n}";
    let reads_module_url =
        "class PageSetup {\n  static configure() { return { base: import.meta.url }; }\n}";
    let differs = "class PageSetup {\n  static configure() { return { caches: {} }; }\n}";
    let cases = [
        (reads_module_class, SetupModuleFixture::PLAIN_SETUP, "[]"),
        (reads_module_url, reads_module_url, "[]"),
        (differs, SetupModuleFixture::PLAIN_SETUP, "[]"),
        (
            SetupModuleFixture::PLAIN_SETUP,
            SetupModuleFixture::PLAIN_SETUP,
            "[{\"name\":\"read\",\"valid\":true}]",
        ),
    ];
    for (alpha_setup, beta_setup, methods) in cases {
        let alpha = SetupModuleFixture::module("AlphaService", alpha_setup, methods);
        let beta = SetupModuleFixture::module("BetaService", beta_setup, methods);
        let sharing = SetupSharing::plan(&[alpha.as_bytes(), beta.as_bytes()]);
        let (without, names) = sharing.without_setups(alpha.as_bytes());
        assert!(names.is_empty(), "{alpha_setup}");
        assert_eq!(String::from_utf8(without).expect("text"), alpha);
        assert!(sharing.exports().is_empty());
    }
}

#[test]
fn a_setup_in_one_module_stays_in_that_module() {
    let alpha = SetupModuleFixture::module("AlphaService", SetupModuleFixture::PLAIN_SETUP, "[]");
    let sharing = SetupSharing::plan(&[alpha.as_bytes()]);
    let (without, names) = sharing.without_setups(alpha.as_bytes());
    assert!(names.is_empty());
    assert_eq!(String::from_utf8(without).expect("text"), alpha);
}

async fn two_services_share_one_setup_in(extension: &str) {
    let language = SetupModuleFixture::language(extension);
    let root = SetupModuleFixture::built(&[language]).await;
    SetupModuleFixture::assert_shared(root.path(), language).await;
}

one_test_per_case!(
    two_script_services_share_one_setup,
    async two_services_share_one_setup_in,
    all SetupModuleFixture::LANGUAGES,
    [javascript => "js", typescript => "ts"]
);

#[tokio::test]
async fn javascript_and_typescript_services_on_one_page_each_share_their_own_setup() {
    let root = SetupModuleFixture::built(&[
        &SetupModuleFixture::JAVASCRIPT,
        &SetupModuleFixture::TYPESCRIPT,
    ])
    .await;
    SetupModuleFixture::assert_shared(root.path(), &SetupModuleFixture::JAVASCRIPT).await;
    SetupModuleFixture::assert_shared(root.path(), &SetupModuleFixture::TYPESCRIPT).await;
}

struct SharedSetup {
    name: &'static str,
    services: [&'static str; 2],
    files: [(&'static str, &'static str); 3],
}

struct SetupModuleFixture;

impl SetupModuleFixture {
    const LANGUAGES: [&'static str; 2] = ["js", "ts"];

    const PLAIN_SETUP: &'static str = "class PageSetup {\n  static configured = 0;\n  static configure() { PageSetup.configured += 1; return {}; }\n}";

    fn module(service: &str, setup: &str, methods: &str) -> String {
        format!(
            "       {setup}\n\nexport default class {service} {{\n  async open(signal) {{ return Behaviors.create(PageSetup, {{partition: 'page', signal}}); }}\n}}\nconst Behaviors = __tachyon_create_behaviors__([{{type:PageSetup,\"cases\":[],\"configuration\":true,\"methods\":{methods},\"name\":\"PageSetup\"}},{{type:{service},\"cases\":[],\"configuration\":false,\"methods\":[],\"name\":\"{service}\"}}], \"tac\", \"{}\");\n",
            "a".repeat(64)
        )
    }

    fn write(root: &Path, path: &str, source: &str) {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directories");
        std::fs::write(path, source).expect("source");
    }

    const JAVASCRIPT: SharedSetup = SharedSetup {
        name: "ScriptSetup",
        services: ["Alpha", "Beta"],
        files: [
            (
                "client/setups/ScriptSetup.js",
                "@Setup\nexport default class ScriptSetup {\n  static configured = 0;\n  static configure() { ScriptSetup.configured += 1; return {}; }\n}",
            ),
            (
                "client/services/AlphaService.js",
                "@Service\nexport default class AlphaService {\n  size = 0;\n  async open(signal) { return [ScriptSetup, await Behaviors.create(ScriptSetup, {partition: 'Alpha', signal})]; }\n}",
            ),
            (
                "client/services/BetaService.js",
                "@Service\nexport default class BetaService {\n  size = 0;\n  async open(signal) { return [ScriptSetup, await Behaviors.create(ScriptSetup, {partition: 'Beta', signal})]; }\n}",
            ),
        ],
    };

    const TYPESCRIPT: SharedSetup = SharedSetup {
        name: "TypedSetup",
        services: ["Gamma", "Delta"],
        files: [
            (
                "client/setups/TypedSetup.ts",
                "@Setup\nexport default class TypedSetup {\n  static configured = 0;\n  static configure(): object { TypedSetup.configured += 1; return {}; }\n}",
            ),
            (
                "client/services/GammaService.ts",
                "@Service\nexport default class GammaService {\n  size: number = 0;\n  async open(signal: AbortSignal): Promise<unknown[]> { return [TypedSetup, await Behaviors.create(TypedSetup, {partition: 'Gamma', signal})]; }\n}",
            ),
            (
                "client/services/DeltaService.ts",
                "@Service\nexport default class DeltaService {\n  size: number = 0;\n  async open(signal: AbortSignal): Promise<unknown[]> { return [TypedSetup, await Behaviors.create(TypedSetup, {partition: 'Delta', signal})]; }\n}",
            ),
        ],
    };

    fn project(root: &Path, languages: &[&SharedSetup]) {
        Self::write(
            root,
            "client/manifest.json",
            "{\"tac\":{\"visibility\":{\"check\":false},\"duplication\":{\"check\":false}}}",
        );
        Self::write(root, "client/pages/tac.html", "<size-panel />");
        let reads = languages
            .iter()
            .flat_map(|language| language.services)
            .map(|service| format!("{{{}.size}}", service.to_ascii_lowercase()))
            .collect::<Vec<_>>();
        Self::write(
            root,
            "client/components/size/panel/tac.html",
            &format!("<p>{}</p>", reads.join(" ")),
        );
        for (path, source) in languages.iter().flat_map(|language| language.files) {
            Self::write(root, path, source);
        }
    }

    fn language(extension: &str) -> &'static SharedSetup {
        match extension {
            "js" => &Self::JAVASCRIPT,
            "ts" => &Self::TYPESCRIPT,
            _ => panic!("no shared setup uses .{extension}"),
        }
    }

    async fn built(languages: &[&SharedSetup]) -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        Self::project(root.path(), languages);
        WebCompiler::build_async(root.path(), &BuildOptions::default())
            .await
            .expect("bundle with two services that open one setup");
        root
    }

    fn built_module(root: &Path, service: &str) -> String {
        let path = root.join(format!("dist/.tachyon/services/{service}Service.js"));
        std::fs::read_to_string(path).expect("service module")
    }

    async fn assert_shared(root: &Path, language: &SharedSetup) {
        let declared = format!("class {}", language.name);
        let [first, second] = language.services;
        let first = Self::built_module(root, first);
        let second = Self::built_module(root, second);
        assert!(!first.contains(&declared), "{first}");
        assert!(!second.contains(&declared), "{second}");
        let runtime = std::fs::read_to_string(root.join("dist/.tachyon/behavior-runtime.js"))
            .expect("runtime");
        assert_eq!(runtime.matches(&format!("export {declared}")).count(), 1);
        Self::invoke(root, language).await;
    }

    async fn invoke(root: &Path, language: &SharedSetup) {
        Self::write(root, "dist/package.json", "{\"type\":\"module\"}");
        let mut command = crate::node_driver::NodeDriver::async_command();
        command
            .args(["--input-type=module", "--eval", &Self::driver(language)])
            .current_dir(root.join("dist"));
        let output = crate::node_driver::NodeDriver::run_within(
            &mut command,
            std::time::Duration::from_secs(30),
        )
        .await
        .expect("supervised Node");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            "one-setup-for-two-services-passed"
        );
    }

    fn driver(language: &SharedSetup) -> String {
        let [first, second] = language.services;
        let setup = language.name;
        format!(
            "import assert from 'node:assert/strict';
const signal = new AbortController().signal;
const opened = [];
for (const name of ['{first}', '{second}']) {{
  const {{default: Service}} = await import(`./.tachyon/services/${{name}}Service.js`);
  opened.push(await new Service().open(signal));
}}
const runtime = await import('./.tachyon/behavior-runtime.js');
assert.equal(opened[0][0], opened[1][0]);
assert.equal(opened[0][0], runtime.{setup});
assert.equal(runtime.{setup}.configured, 1);
assert.notEqual(opened[0][1], opened[1][1]);
console.log('one-setup-for-two-services-passed');"
        )
    }
}
