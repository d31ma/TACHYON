use super::shared_assets::SharedAssets;
use crate::Failure;
use std::path::Path;

struct SharedScriptFixture;

impl SharedScriptFixture {
    const VENDOR: &'static str = "document.write('<vendor-panel />');\nimport './vendor.css';\n";
    const URL: &'static str = "/shared/vendor/widget.js";

    fn write(root: &Path, path: &str, source: &str) {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().unwrap_or(root))
            .unwrap_or_else(|error| panic!("directory: {error}"));
        std::fs::write(path, source).unwrap_or_else(|error| panic!("source: {error}"));
    }

    fn manifest(root: &Path, declared: bool, digest: &str) {
        let scripts = declared
            .then_some(Self::URL)
            .into_iter()
            .collect::<Vec<_>>();
        let source = serde_json::json!({"tac": {"scripts": scripts, "scriptSources": { (Self::URL): {"package": "test-widget", "version": "1.0.0", "license": "MIT", "sha256": digest} }}});
        Self::write(root, "client/manifest.json", &source.to_string());
    }

    fn refuse(root: &Path, code: &str, path: &str) -> Failure {
        let failure = SharedAssets::collect(root)
            .err()
            .unwrap_or_else(|| panic!("accepted copied sink"));
        assert!(failure.to_string().contains(code), "{failure}");
        assert!(
            failure
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.spans.iter().any(|span| span.file == path)),
            "{failure}"
        );
        failure
    }
}

#[test]
fn authored_dom_guard_checks_raw_unreachable_shared_scripts_before_rewrite() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    let source = "import '../site.css';\nconst node = document.createElement('article'); document.body.append(node);";
    SharedScriptFixture::write(root.path(), "client/shared/assets/renamed.mjs", source);
    let failure =
        SharedScriptFixture::refuse(root.path(), "TY1364", "client/shared/assets/renamed.mjs");
    let span = &failure.diagnostics()[0].spans[0];
    let start = usize::try_from(span.start).unwrap_or_else(|error| panic!("span start: {error}"));
    let end = usize::try_from(span.end).unwrap_or_else(|error| panic!("span end: {error}"));
    assert!(source[start..end].contains("append"));
}

#[test]
fn authored_dom_guard_does_not_exempt_copied_runtime_names_or_role_renames() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    SharedScriptFixture::write(
        root.path(),
        "client/shared/assets/tac-client.js",
        "element.innerHTML = html;",
    );
    SharedScriptFixture::refuse(root.path(), "TY1364", "client/shared/assets/tac-client.js");
}

#[test]
fn authored_dom_guard_preserves_exact_verified_declared_vendor_bytes() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    SharedScriptFixture::write(
        root.path(),
        "client/shared/vendor/widget.js",
        SharedScriptFixture::VENDOR,
    );
    let digest = super::ContentDigest::sha256_bytes(SharedScriptFixture::VENDOR.as_bytes());
    SharedScriptFixture::manifest(root.path(), true, &digest);
    let assets = SharedAssets::collect(root.path())
        .unwrap_or_else(|failure| panic!("verified vendor: {failure}"));
    let bytes = assets
        .iter()
        .find(|(path, _)| path == Path::new("shared/vendor/widget.js"))
        .map(|(_, bytes)| bytes);
    assert_eq!(
        bytes.map(Vec::as_slice),
        Some(SharedScriptFixture::VENDOR.as_bytes())
    );
}

#[test]
fn authored_dom_guard_requires_vendor_hash_and_document_resource_declaration() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    SharedScriptFixture::write(
        root.path(),
        "client/shared/vendor/widget.js",
        SharedScriptFixture::VENDOR,
    );
    SharedScriptFixture::manifest(root.path(), true, &"0".repeat(64));
    SharedScriptFixture::refuse(root.path(), "TY1403", "client/shared/vendor/widget.js");
    let digest = super::ContentDigest::sha256_bytes(SharedScriptFixture::VENDOR.as_bytes());
    SharedScriptFixture::manifest(root.path(), false, &digest);
    SharedScriptFixture::refuse(root.path(), "TY1365", "client/shared/vendor/widget.js");
}

#[test]
fn authored_dom_guard_vendor_name_and_metadata_cannot_hide_missing_source() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    SharedScriptFixture::manifest(root.path(), true, &"0".repeat(64));
    SharedScriptFixture::refuse(root.path(), "TY1365", "client/shared/vendor/widget.js");
}

#[test]
fn authored_dom_guard_keeps_runtime_emission_outside_shared_authored_capture() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    SharedScriptFixture::write(
        root.path(),
        "dist/.tachyon/tac-client.js",
        "element.innerHTML = html;",
    );
    SharedScriptFixture::write(
        root.path(),
        "client/shared/data/example.json",
        "{\"example\":\"element.innerHTML = html\"}",
    );
    let assets = SharedAssets::collect(root.path())
        .unwrap_or_else(|failure| panic!("data and separate runtime: {failure}"));
    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].0, Path::new("shared/data/example.json"));
}

#[test]
fn authored_dom_guard_checks_original_component_companion_before_lowering() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    let failure = super::component_script::ComponentScript::prepare_captured(
        root.path(),
        b"element.innerHTML = html;".to_vec(),
        "client/components/panel/tac.js",
    )
    .err()
    .unwrap_or_else(|| panic!("accepted component sink"));
    assert!(failure.to_string().contains("TY1364"), "{failure}");
}

#[tokio::test]
async fn authored_dom_guard_checks_reachable_module_before_transpilation() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    SharedScriptFixture::write(
        root.path(),
        "client/shared/scripts/renamed.js",
        "document.write(markup);",
    );
    let mut assets = super::script_assets::ScriptAssets::new(root.path(), root.path());
    assets
        .rewrite(
            Path::new("client/services/ScreenService.js"),
            Path::new(".tachyon/services/ScreenService.js"),
            b"import '/shared/scripts/renamed.js';".to_vec(),
        )
        .unwrap_or_else(|failure| panic!("captured import: {failure}"));
    let failure = assets
        .collect()
        .await
        .err()
        .unwrap_or_else(|| panic!("accepted imported sink"));
    assert!(failure.to_string().contains("TY1364"), "{failure}");
    assert_eq!(
        failure.diagnostics()[0].spans[0].file,
        "client/shared/scripts/renamed.js"
    );
}

struct CapturedVendorFixture;

impl CapturedVendorFixture {
    const SOURCE: &'static str = "const panel = document.createElement('section'); panel.innerHTML = '<p>Third-party panel</p>'; document.body.append(panel);";

    fn project(root: &Path, source: &str, declared: bool, digest: &str) {
        SharedScriptFixture::write(root, "client/pages/tac.html", "<vendor-panel />");
        SharedScriptFixture::write(
            root,
            "client/components/vendor-panel/tac.html",
            "<p>Vendor fixture</p>",
        );
        SharedScriptFixture::write(root, "client/shared/vendor/widget.js", source);
        SharedScriptFixture::manifest(root, declared, digest);
    }

    fn digest(source: &str) -> String {
        super::ContentDigest::sha256_bytes(source.as_bytes())
    }
}

#[tokio::test]
async fn authored_dom_guard_ordinary_build_accepts_exact_captured_vendor() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    CapturedVendorFixture::project(
        root.path(),
        CapturedVendorFixture::SOURCE,
        true,
        &CapturedVendorFixture::digest(CapturedVendorFixture::SOURCE),
    );
    super::WebCompiler::build_async(root.path(), &super::BuildOptions::default())
        .await
        .unwrap_or_else(|failure| panic!("ordinary pinned vendor build: {failure}"));
    let emitted = std::fs::read(root.path().join("dist/shared/vendor/widget.js"))
        .unwrap_or_else(|error| panic!("vendor artifact: {error}"));
    assert_eq!(emitted, CapturedVendorFixture::SOURCE.as_bytes());
}

#[tokio::test]
async fn authored_dom_guard_ordinary_build_refuses_changed_vendor_hash() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    CapturedVendorFixture::project(
        root.path(),
        CapturedVendorFixture::SOURCE,
        true,
        &"0".repeat(64),
    );
    let failure = super::WebCompiler::build_async(root.path(), &super::BuildOptions::default())
        .await
        .err()
        .unwrap_or_else(|| panic!("accepted mismatched vendor"));
    assert!(failure.to_string().contains("TY1403"), "{failure}");
    assert!(failure.diagnostics().iter().any(|problem| {
        problem
            .spans
            .iter()
            .any(|span| span.file == "client/shared/vendor/widget.js")
    }));
}

#[tokio::test]
async fn authored_dom_guard_ordinary_build_refuses_unlisted_vendor_and_utility_escape() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    CapturedVendorFixture::project(
        root.path(),
        CapturedVendorFixture::SOURCE,
        false,
        &CapturedVendorFixture::digest(CapturedVendorFixture::SOURCE),
    );
    let failure = super::WebCompiler::build_async(root.path(), &super::BuildOptions::default())
        .await
        .err()
        .unwrap_or_else(|| panic!("accepted unlisted vendor"));
    assert!(failure.to_string().contains("TY1365"), "{failure}");
    let utility = "class MarkupUtils { static write() { document.write(markup); } }";
    SharedScriptFixture::write(root.path(), "client/shared/utils/MarkupUtils.js", utility);
    let manifest = serde_json::json!({"tac":{"scripts":["/shared/utils/MarkupUtils.js"],"scriptSources":{"/shared/utils/MarkupUtils.js":{"package":"test-widget","version":"1.0.0","license":"MIT","sha256":CapturedVendorFixture::digest(utility)}}}});
    SharedScriptFixture::write(root.path(), "client/manifest.json", &manifest.to_string());
    let failure = super::WebCompiler::build_async(root.path(), &super::BuildOptions::default())
        .await
        .err()
        .unwrap_or_else(|| panic!("accepted utility vendor escape"));
    assert!(
        failure.to_string().contains("registered utility folders"),
        "{failure}"
    );
}

#[tokio::test]
async fn authored_dom_guard_captured_vendor_cannot_hide_registered_role_or_eval() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    let role = "@Service\nclass WidgetService { text = 'service'; }";
    CapturedVendorFixture::project(
        root.path(),
        role,
        true,
        &CapturedVendorFixture::digest(role),
    );
    let failure = super::WebCompiler::build_async(root.path(), &super::BuildOptions::default())
        .await
        .err()
        .unwrap_or_else(|| panic!("accepted misplaced registered role"));
    assert!(failure.to_string().contains("TY1310"), "{failure}");
    let eval = "eval(source); document.body.innerHTML = markup;";
    CapturedVendorFixture::project(
        root.path(),
        eval,
        true,
        &CapturedVendorFixture::digest(eval),
    );
    let failure = super::WebCompiler::build_async(root.path(), &super::BuildOptions::default())
        .await
        .err()
        .unwrap_or_else(|| panic!("accepted vendor eval"));
    assert!(failure.to_string().contains("TY1325"), "{failure}");
}

#[tokio::test]
async fn authored_dom_guard_vendor_context_uses_captured_manifest_and_source_bytes() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    CapturedVendorFixture::project(
        root.path(),
        CapturedVendorFixture::SOURCE,
        true,
        &CapturedVendorFixture::digest(CapturedVendorFixture::SOURCE),
    );
    let project = crate::ProjectDiscovery::discover(root.path())
        .unwrap_or_else(|failure| panic!("verified capture: {failure}"));
    SharedScriptFixture::manifest(root.path(), true, &"0".repeat(64));
    SharedScriptFixture::write(
        root.path(),
        "client/shared/vendor/widget.js",
        "document.write('changed after capture');",
    );
    super::WebCompiler::build_project_async(&project, &super::BuildOptions::default())
        .await
        .unwrap_or_else(|failure| panic!("immutable captured vendor build: {failure}"));
    let emitted = std::fs::read(root.path().join("dist/shared/vendor/widget.js"))
        .unwrap_or_else(|error| panic!("vendor artifact: {error}"));
    assert_eq!(emitted, CapturedVendorFixture::SOURCE.as_bytes());
    assert!(
        crate::ProjectDiscovery::discover(root.path()).is_err(),
        "changed live declaration/source cannot qualify a fresh capture"
    );
}
