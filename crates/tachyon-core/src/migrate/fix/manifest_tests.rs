use super::{MigrationFix, MigrationFixPlan};
use serde_json::{Value, json};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn project(files: &[(&str, &str)]) -> TempDir {
    let root = tempfile::tempdir().expect("project");
    for (relative, source) in files {
        let path = root.path().join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        fs::write(path, source).expect("file");
    }
    root
}

fn plan(root: &Path) -> MigrationFixPlan {
    MigrationFix::plan(root).expect("plan")
}

fn manifest(root: &Path) -> Value {
    let text = fs::read_to_string(root.join("client/manifest.json")).expect("manifest");
    serde_json::from_str(&text).expect("manifest JSON")
}

fn manual_text(plan: &MigrationFixPlan) -> String {
    let mut text = String::new();
    for step in &plan.manual_steps {
        let _ = writeln!(text, "{} | {} | {}", step.source, step.problem, step.action);
    }
    text
}

fn write_and_plan_again(root: &Path) -> MigrationFixPlan {
    plan(root).write(root).expect("write");
    plan(root)
}

struct Fixture;

impl Fixture {
    const LEGACY: &str = r#"{"application":{"name":"Shop","id":"dev.example.shop","version":"1.2.3","entry_route":"/"}}"#;
}

#[test]
fn a_project_without_old_configuration_has_nothing_to_move() {
    let root = project(&[("client/manifest.json", r#"{"name":"Shop"}"#)]);
    let planned = plan(root.path());
    assert_eq!(planned.change_count(), 0);
    assert_eq!(planned.delete_count(), 0);
    assert_eq!(planned.manual_step_count(), 0);
}

#[test]
fn tachyon_json_moves_into_the_manifest_and_is_deleted() {
    let root = project(&[
        (
            "client/manifest.json",
            "{\n  \"name\": \"Shop\",\n  \"display\": \"standalone\"\n}\n",
        ),
        ("tachyon.json", Fixture::LEGACY),
    ]);
    let planned = plan(root.path());
    assert_eq!(planned.change_count(), 1);
    assert_eq!(planned.delete_count(), 1);
    assert!(planned.to_text(false).contains("delete   tachyon.json"));
    let again = write_and_plan_again(root.path());
    assert!(!root.path().join("tachyon.json").exists());
    assert_eq!(
        fs::read_to_string(root.path().join("client/manifest.json")).expect("manifest"),
        "{\n  \"name\": \"Shop\",\n  \"display\": \"standalone\",\n  \"start_url\": \"/\",\n  \"tac\": {\n    \"application\": {\n      \"id\": \"dev.example.shop\",\n      \"version\": \"1.2.3\"\n    }\n  }\n}\n"
    );
    assert_eq!(again.change_count(), 0);
    assert_eq!(again.delete_count(), 0);
    assert_eq!(again.manual_step_count(), 0);
}

#[test]
fn a_missing_manifest_is_created() {
    let root = project(&[("tachyon.json", Fixture::LEGACY)]);
    write_and_plan_again(root.path());
    assert_eq!(
        manifest(root.path()),
        json!({
            "name": "Shop",
            "start_url": "/",
            "tac": { "application": { "id": "dev.example.shop", "version": "1.2.3" } },
        })
    );
    assert!(!root.path().join("tachyon.json").exists());
}

#[test]
fn a_different_value_keeps_tachyon_json_and_asks_for_a_choice() {
    let root = project(&[
        (
            "client/manifest.json",
            r#"{"name":"Store","tac":{"application":{"id":"dev.example.store"}}}"#,
        ),
        ("tachyon.json", Fixture::LEGACY),
    ]);
    let planned = plan(root.path());
    assert_eq!(planned.delete_count(), 0);
    let text = manual_text(&planned);
    assert!(text.contains("sets name to \"Shop\""), "{text}");
    assert!(text.contains("tac.application"), "{text}");
    assert!(text.contains("then delete tachyon.json"), "{text}");
    planned.write(root.path()).expect("write");
    assert!(root.path().join("tachyon.json").exists());
    assert_eq!(manifest(root.path())["start_url"], json!("/"));
}

#[test]
fn a_changed_tachyon_json_is_not_deleted() {
    let root = project(&[
        ("client/manifest.json", "{}"),
        ("tachyon.json", Fixture::LEGACY),
    ]);
    let planned = plan(root.path());
    fs::write(root.path().join("tachyon.json"), "{}").expect("edit");
    let error = planned.write(root.path()).expect_err("changed file");
    assert!(
        error
            .to_string()
            .contains("changed after the fix was planned")
    );
    assert_eq!(
        fs::read_to_string(root.path().join("client/manifest.json")).expect("manifest"),
        "{}"
    );
}

#[test]
fn plain_tac_config_values_move_and_code_becomes_manual_steps() {
    let configuration = "import { writeFile } from 'node:fs/promises'\n\
        export default class Configuration {\n\
          static origin = process.env.SITE_ORIGIN ?? 'https://shop.example'\n\
          static application = { name: 'Shop', id: 'dev.example.shop', version: '1.2.3', entryRoute: '/' }\n\
          static scripts = ['/shared/app.js'] // loaded everywhere\n\
          static styles = [\"/shared/app.css\",]\n\
          static cache = [{ path: '/api/*', policy: 'network-first' }];\n\
          static metadata = {\n\
            '/': { title: 'Shop \\u2014 home', noindex: false },\n\
          }\n\
          static async postBundle({ targetRoots }) {\n\
            await writeFile(`${targetRoots.web}/robots.txt`, '')\n\
          }\n\
        }\n";
    let root = project(&[
        (
            "client/manifest.json",
            "{\n  \"name\": \"Shop\",\n  \"tac\": {\n    \"api\": { \"url\": \"/api\" }\n  }\n}\n",
        ),
        ("tac.config.js", configuration),
    ]);
    let planned = plan(root.path());
    let text = manual_text(&planned);
    assert!(text.contains("static origin is computed"), "{text}");
    assert!(text.contains("tac.origin"), "{text}");
    assert!(text.contains("postBundle is removed"), "{text}");
    assert!(text.contains("then delete it"), "{text}");
    let again = write_and_plan_again(root.path());
    let updated = manifest(root.path());
    assert_eq!(updated["start_url"], json!("/"));
    assert_eq!(
        updated["tac"],
        json!({
            "api": { "url": "/api" },
            "application": { "id": "dev.example.shop", "version": "1.2.3" },
            "scripts": ["/shared/app.js"],
            "styles": ["/shared/app.css"],
            "cache": [{ "path": "/api/*", "policy": "network-first" }],
            "metadata": { "routes": { "/": { "title": "Shop \u{2014} home", "noindex": false } } },
        })
    );
    assert!(root.path().join("tac.config.js").exists());
    assert_eq!(again.change_count(), 0);
    assert_eq!(manual_text(&again), text);
}

#[test]
fn computed_values_are_never_copied() {
    for (field, declaration) in [
        ("scripts", "static scripts = Configuration.list()"),
        ("styles", "static styles = ['/a.css', ...more]"),
        (
            "metadata",
            "static metadata = { '/': { canonical: `${Configuration.origin}/` } }",
        ),
        ("cache", "static cache() { return [] }"),
        ("application", "static application = { name: 'A' + 'B' }"),
    ] {
        let source = format!("export default class Configuration {{\n  {declaration}\n}}\n");
        let root = project(&[("tac.config.ts", &source)]);
        let planned = plan(root.path());
        assert_eq!(planned.change_count(), 0, "{declaration}");
        let text = manual_text(&planned);
        assert!(
            text.contains(&format!("static {field} is computed")),
            "{declaration}: {text}"
        );
    }
}

#[test]
fn the_old_tachyon_member_is_a_manual_step() {
    let root = project(&[(
        "client/manifest.json",
        r#"{"tachyon":{"id":"dev.example.shop"}}"#,
    )]);
    let text = manual_text(&plan(root.path()));
    assert!(text.contains("moved to tac.application"), "{text}");
}

#[test]
fn invalid_manifest_json_is_reported_before_anything_moves() {
    let root = project(&[
        ("client/manifest.json", "{"),
        ("tachyon.json", Fixture::LEGACY),
    ]);
    let planned = plan(root.path());
    assert_eq!(planned.change_count(), 0);
    assert_eq!(planned.delete_count(), 0);
    assert!(manual_text(&planned).contains("not valid JSON"));
}
