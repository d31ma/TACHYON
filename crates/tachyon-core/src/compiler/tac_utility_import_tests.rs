use super::{BuildOptions, WebCompiler};
use std::path::Path;

struct TacUtilityImportTests;

impl TacUtilityImportTests {
    const SOURCE: &'static str = "class TitleUtils { static title() { return 'Ready'; } static location() { return import.meta.url; } }";

    fn write(root: &Path, relative: &str, source: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap_or_else(|| panic!("parent")))
            .unwrap_or_else(|failure| panic!("source directory: {failure}"));
        std::fs::write(path, source).unwrap_or_else(|failure| panic!("source: {failure}"));
    }

    fn project(import: &str, utility: &str) -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap_or_else(|failure| panic!("project: {failure}"));
        Self::write(root.path(), "client/pages/tac.html", "<title-panel />");
        Self::write(
            root.path(),
            "client/components/title/panel/tac.html",
            "<p>{label.text}</p>",
        );
        Self::write(
            root.path(),
            "client/shared/utils/text/TitleUtils.js",
            utility,
        );
        Self::write(
            root.path(),
            "client/services/nested/LabelService.js",
            &format!(
                "{import}\n@Service\nexport default class LabelService {{ text = TitleUtils.title(); location = TitleUtils.location(); }}"
            ),
        );
        root
    }

    async fn build_and_call(root: &Path) {
        let result = WebCompiler::build_async(root, &BuildOptions::default())
            .await
            .unwrap_or_else(|failure| panic!("ordinary shared utility bundle: {failure}"));
        let output = result.output_directory();
        Self::write(output, "package.json", "{\"type\":\"module\"}");
        let mut command = tokio::process::Command::new("node");
        command
            .args([
                "--input-type=module",
                "--eval",
                "import assert from 'node:assert/strict'; const {default: LabelService} = await import('./.tachyon/services/LabelService.js'); const label = new LabelService(); assert.equal(label.text, 'Ready'); assert.ok(new URL(label.location).pathname.endsWith('/shared/utils/text/TitleUtils.js'));",
            ])
            .current_dir(output);
        let captured =
            crate::external_command::run(&mut command, std::time::Duration::from_secs(30), 65_536)
                .await
                .unwrap_or_else(|failure| panic!("supervised emitted module execution: {failure}"));
        assert!(
            captured.status.success(),
            "{}",
            String::from_utf8_lossy(&captured.stderr)
        );
    }
}

#[tokio::test]
async fn tac_utility_import_canonical_absolute_module_runs_without_duplicate_inline_binding() {
    let root = TacUtilityImportTests::project(
        "import { TitleUtils } from '/shared/utils/text/TitleUtils.js';",
        TacUtilityImportTests::SOURCE,
    );
    TacUtilityImportTests::build_and_call(root.path()).await;
}

#[tokio::test]
async fn tac_utility_import_nested_relative_module_runs_without_duplicate_inline_binding() {
    let root = TacUtilityImportTests::project(
        "import { TitleUtils } from '../../shared/utils/text/TitleUtils.js';",
        TacUtilityImportTests::SOURCE,
    );
    TacUtilityImportTests::build_and_call(root.path()).await;
}

#[tokio::test]
async fn tac_utility_import_implicit_realm_utility_keeps_real_module_identity() {
    let root = TacUtilityImportTests::project("", TacUtilityImportTests::SOURCE);
    TacUtilityImportTests::build_and_call(root.path()).await;
}

#[tokio::test]
async fn tac_utility_import_implicit_typescript_utility_runs_from_a_typescript_service() {
    let root = TacUtilityImportTests::project("", TacUtilityImportTests::SOURCE);
    std::fs::remove_file(root.path().join("client/shared/utils/text/TitleUtils.js"))
        .unwrap_or_else(|failure| panic!("javascript utility: {failure}"));
    std::fs::remove_file(root.path().join("client/services/nested/LabelService.js"))
        .unwrap_or_else(|failure| panic!("javascript service: {failure}"));
    TacUtilityImportTests::write(
        root.path(),
        "client/shared/utils/text/TitleUtils.ts",
        "class TitleUtils { static title(): string { return 'Ready'; } static location(): string { return import.meta.url; } }",
    );
    TacUtilityImportTests::write(
        root.path(),
        "client/services/nested/LabelService.ts",
        "@Service\nexport default class LabelService { text: string = TitleUtils.title(); location: string = TitleUtils.location(); }",
    );
    TacUtilityImportTests::build_and_call(root.path()).await;
}

#[tokio::test]
async fn tac_utility_import_mjs_cannot_skip_the_shared_utility_shape_guard() {
    let root = TacUtilityImportTests::project(
        "import { TitleUtils } from '/shared/utils/text/UncheckedUtils.mjs';",
        TacUtilityImportTests::SOURCE,
    );
    TacUtilityImportTests::write(
        root.path(),
        "client/shared/utils/text/UncheckedUtils.mjs",
        "export class TitleUtils { static value = 1; }",
    );
    let failure = WebCompiler::build_async(root.path(), &BuildOptions::default())
        .await
        .err()
        .unwrap_or_else(|| panic!("unchecked mjs utility must be refused"));
    assert!(
        failure
            .to_string()
            .contains("utility modules use .js or .ts")
    );
    assert!(failure.to_string().contains("UncheckedUtils.mjs"));
}
