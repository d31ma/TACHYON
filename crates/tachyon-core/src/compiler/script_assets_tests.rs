#![allow(clippy::expect_used)]

use super::{BuildOptions, WebCompiler};
use std::path::Path;

struct ScriptAssetFixture;

impl ScriptAssetFixture {
    fn write(root: &Path, path: &str, source: &str) {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directories");
        std::fs::write(path, source).expect("source");
    }

    fn project(root: &Path, import: &str) {
        Self::write(root, "client/pages/tac.html", "<label-panel />");
        Self::write(
            root,
            "client/components/label-panel/tac.html",
            "<main>{label.text}</main>",
        );
        Self::write(
            root,
            "client/services/nested/LabelService.js",
            &format!(
                "import {{ TitleUtils }} from '{import}';\n@Service\nexport default class LabelService {{ text = TitleUtils.text(); }}"
            ),
        );
        Self::write(
            root,
            "client/utils/nested/TitleUtils.js",
            "import { TextSource } from '../../shared/scripts/text.js';\nimport './CycleUtils.js';\nexport class TitleUtils { static text() { return TextSource.text(); } }",
        );
        Self::write(
            root,
            "client/utils/nested/CycleUtils.js",
            "import { TitleUtils } from './TitleUtils.js';\nexport class CycleUtils {}",
        );
        Self::write(
            root,
            "client/shared/scripts/text.js",
            "import '../site.css';\nimport { EndingUtils } from '/utils/EndingUtils.js';\nexport class TextSource { static text() { return EndingUtils.text(); } }",
        );
        Self::write(
            root,
            "client/utils/EndingUtils.js",
            "export class EndingUtils { static text() { return 'Ready'; } }",
        );
        Self::write(
            root,
            "client/utils/UnusedUtils.js",
            "export class UnusedUtils { static _text() { return 'unused'; } }",
        );
    }

    async fn invoke(root: &Path) {
        Self::write(root, "dist/package.json", "{\"type\":\"module\"}");
        let mut command = tokio::process::Command::new("node");
        command.args([
            "--input-type=module",
            "--eval",
            "const { default: LabelService } = await import('./.tachyon/services/LabelService.js'); if (new LabelService().text !== 'Ready') throw new Error('wrong utility result');",
        ]).current_dir(root.join("dist"));
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
    }
}

#[tokio::test]
#[ignore = "requires TAC_SCRIPT_ASSET_BUNDLE pointing to a completed CLI web bundle"]
async fn script_assets_completed_native_bundle_executes_authored_utility() {
    let root = std::env::var_os("TAC_SCRIPT_ASSET_BUNDLE").expect("completed bundle path");
    let mut command = tokio::process::Command::new("node");
    command.args([
        "--input-type=module",
        "--eval",
        "const { default: LabelService } = await import('./tachyon-runtime/services/LabelService.js'); if (new LabelService().text !== 'Ready') throw new Error('wrong utility result'); console.log('LabelService.text=Ready');",
    ]).current_dir(root);
    let output =
        crate::external_command::run(&mut command, std::time::Duration::from_secs(30), 64 * 1024)
            .await
            .expect("supervised Node");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "LabelService.text=Ready"
    );
}

#[tokio::test]
async fn script_assets_publish_reachable_nested_transitive_utilities_with_cycles() {
    for import in [
        "../../utils/nested/TitleUtils.js",
        "/utils/nested/TitleUtils.js",
        "../../utils/nested/TitleUtils.js?v=1#loaded",
    ] {
        let root = tempfile::tempdir().expect("project");
        ScriptAssetFixture::project(root.path(), import);
        WebCompiler::build_async(root.path(), &BuildOptions::default())
            .await
            .expect("build");
        assert!(
            root.path()
                .join("dist/utils/nested/TitleUtils.js")
                .is_file()
        );
        assert!(
            root.path()
                .join("dist/utils/nested/CycleUtils.js")
                .is_file()
        );
        assert!(!root.path().join("dist/utils/UnusedUtils.js").exists());
        ScriptAssetFixture::invoke(root.path()).await;
    }
}

#[tokio::test]
async fn script_assets_refuse_missing_dependencies_before_publication() {
    let root = tempfile::tempdir().expect("project");
    ScriptAssetFixture::project(root.path(), "/utils/MissingUtils.js");
    let failure = WebCompiler::build_async(root.path(), &BuildOptions::default())
        .await
        .expect_err("missing utility")
        .to_string();
    assert!(failure.contains("TY1201"), "{failure}");
    assert!(
        failure.contains("client/utils/MissingUtils.js"),
        "{failure}"
    );
    assert!(!root.path().join("dist/index.html").exists());
}

#[tokio::test]
async fn script_assets_compile_typed_utilities_imported_with_javascript_names() {
    let root = tempfile::tempdir().expect("project");
    ScriptAssetFixture::project(root.path(), "/utils/TypedUtils.js");
    ScriptAssetFixture::write(
        root.path(),
        "client/utils/TypedUtils.ts",
        "export class TypedUtils { static text(): string { return 'Ready'; } }",
    );
    let service = root.path().join("client/services/nested/LabelService.js");
    let source = std::fs::read_to_string(&service).expect("service");
    let source = source.replace("TitleUtils", "TypedUtils").replace(
        "TypedUtils.text()",
        "TitleUtils.text().replace('Ready', TypedUtils.text())",
    );
    let source = format!("import {{ TitleUtils }} from '/utils/nested/TitleUtils.js';\n{source}");
    std::fs::write(service, source).expect("typed import");
    WebCompiler::build_async(root.path(), &BuildOptions::default())
        .await
        .expect("typed build");
    assert!(root.path().join("dist/utils/TypedUtils.js").is_file());
    assert!(!root.path().join("dist/utils/TypedUtils.ts").exists());
    ScriptAssetFixture::invoke(root.path()).await;
}

#[test]
fn script_assets_refuse_unchecked_mjs_utilities() {
    let root = tempfile::tempdir().expect("project");
    ScriptAssetFixture::write(
        root.path(),
        "client/utils/TitleUtils.mjs",
        "export class TitleUtils {}",
    );
    let mut assets = super::script_assets::ScriptAssets::new(root.path(), root.path());
    let failure = assets
        .rewrite(
            Path::new("client/services/LabelService.js"),
            Path::new(".tachyon/services/LabelService.js"),
            b"import { TitleUtils } from '../utils/TitleUtils.mjs';".to_vec(),
        )
        .expect_err("mjs bypass refused")
        .to_string();
    assert!(failure.contains(".js or .ts"), "{failure}");
}
