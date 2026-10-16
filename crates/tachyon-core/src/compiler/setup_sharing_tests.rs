#![allow(clippy::expect_used)]

use super::setup_sharing::SetupSharing;
use super::{BuildOptions, WebCompiler};
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

#[tokio::test]
async fn two_built_services_share_one_setup_class_and_one_configuration() {
    let root = tempfile::tempdir().expect("project");
    SetupModuleFixture::project(root.path());
    WebCompiler::build_async(root.path(), &BuildOptions::default())
        .await
        .expect("bundle with two services for each script language");
    for service in [
        "AlphaService",
        "BetaService",
        "GammaService",
        "DeltaService",
    ] {
        let path = root
            .path()
            .join(format!("dist/.tachyon/services/{service}.js"));
        let module = std::fs::read_to_string(path).expect("service module");
        assert!(!module.contains("class ScriptSetup"), "{module}");
        assert!(!module.contains("class TypedSetup"), "{module}");
    }
    let runtime = std::fs::read_to_string(root.path().join("dist/.tachyon/behavior-runtime.js"))
        .expect("runtime");
    assert_eq!(runtime.matches("export class ScriptSetup").count(), 1);
    assert_eq!(runtime.matches("export class TypedSetup").count(), 1);
    SetupModuleFixture::invoke(root.path()).await;
}

struct SetupModuleFixture;

impl SetupModuleFixture {
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

    fn project(root: &Path) {
        Self::write(
            root,
            "client/manifest.json",
            "{\"tac\":{\"visibility\":{\"check\":false},\"duplication\":{\"check\":false}}}",
        );
        Self::write(root, "client/pages/tac.html", "<size-panel />");
        Self::write(
            root,
            "client/components/size/panel/tac.html",
            "<p>{alpha.size} {beta.size} {gamma.size} {delta.size}</p>",
        );
        Self::write(
            root,
            "client/setups/ScriptSetup.js",
            "@Setup\nexport default class ScriptSetup {\n  static configured = 0;\n  static configure() { ScriptSetup.configured += 1; return {}; }\n}",
        );
        Self::write(
            root,
            "client/setups/TypedSetup.ts",
            "@Setup\nexport default class TypedSetup {\n  static configured = 0;\n  static configure(): object { TypedSetup.configured += 1; return {}; }\n}",
        );
        for name in ["Alpha", "Beta"] {
            Self::write(
                root,
                &format!("client/services/{name}Service.js"),
                &format!(
                    "@Service\nexport default class {name}Service {{\n  size = 0;\n  async open(signal) {{ return [ScriptSetup, await Behaviors.create(ScriptSetup, {{partition: '{name}', signal}})]; }}\n}}"
                ),
            );
        }
        for name in ["Gamma", "Delta"] {
            Self::write(
                root,
                &format!("client/services/{name}Service.ts"),
                &format!(
                    "@Service\nexport default class {name}Service {{\n  size: number = 0;\n  async open(signal: AbortSignal): Promise<unknown[]> {{ return [TypedSetup, await Behaviors.create(TypedSetup, {{partition: '{name}', signal}})]; }}\n}}"
                ),
            );
        }
    }

    async fn invoke(root: &Path) {
        Self::write(root, "dist/package.json", "{\"type\":\"module\"}");
        let mut command = tokio::process::Command::new("node");
        command
            .args([
                "--input-type=module",
                "--eval",
                r"import assert from 'node:assert/strict';
const signal = new AbortController().signal;
const opened = [];
for (const name of ['Alpha', 'Beta', 'Gamma', 'Delta']) {
  const {default: Service} = await import(`./.tachyon/services/${name}Service.js`);
  opened.push(await new Service().open(signal));
}
const runtime = await import('./.tachyon/behavior-runtime.js');
assert.equal(opened[0][0], opened[1][0]);
assert.equal(opened[0][0], runtime.ScriptSetup);
assert.equal(opened[2][0], opened[3][0]);
assert.equal(opened[2][0], runtime.TypedSetup);
assert.equal(runtime.ScriptSetup.configured, 1);
assert.equal(runtime.TypedSetup.configured, 1);
assert.notEqual(opened[0][1], opened[1][1]);
console.log('one-setup-for-two-services-passed');",
            ])
            .current_dir(root.join("dist"));
        let output = crate::external_command::run(
            &mut command,
            std::time::Duration::from_secs(30),
            64 * 1024,
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
}
