#![allow(clippy::expect_used)]

use super::{BuildOptions, WebCompiler};
use std::path::Path;

struct ModelScriptFixture;

impl ModelScriptFixture {
    fn write(root: &Path, path: &str, source: &str) {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directories");
        std::fs::write(path, source).expect("source");
    }

    fn project(root: &Path, extension: &str) {
        Self::write(root, "client/pages/tac.html", "<input-panel />");
        Self::write(
            root,
            "client/components/input-panel/tac.html",
            "<main>{input.calls}</main>",
        );
        let field = ["name;", "name: string;"][usize::from(extension == "ts")];
        Self::write(
            root,
            &format!("client/models/forms/ProductInput.{extension}"),
            &format!("@Checked\nexport default class ProductInput {{\n  @Required\n  {field}\n}}"),
        );
        let parameter = ["input", "input: ProductInput"][usize::from(extension == "ts")];
        let import = [
            "/** @import { ProductInput } from '/models/forms/ProductInput.js' */",
            "import type ProductInput from '/models/forms/ProductInput.ts';",
        ][usize::from(extension == "ts")];
        Self::write(
            root,
            &format!("client/services/forms/InputService.{extension}"),
            &format!(
                "{import}\n@Service\nexport default class InputService {{\n  calls = 0;\n  /** @param {{ProductInput}} input */\n  async submit(@Check {parameter}) {{ this.calls++; return input; }}\n}}"
            ),
        );
    }

    async fn invoke(root: &Path) {
        Self::write(root, "dist/package.json", "{\"type\":\"module\"}");
        let mut command = tokio::process::Command::new("node");
        command.args([
            "--input-type=module", "--eval",
            "const {default: InputService} = await import('./.tachyon/services/InputService.js'); const service = new InputService(); const accepted = await service.submit({name: 'Ready'}); if (accepted.name !== 'Ready' || service.calls !== 1) throw new Error('valid input lost'); let denied = false; try { await service.submit({}); } catch (error) { denied = error.code === 'validation'; } if (!denied || service.calls !== 1) throw new Error('invalid input reached service');",
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
async fn script_model_schemas_validate_real_tac_calls_after_emission() {
    for extension in ["js", "ts"] {
        let root = tempfile::tempdir().expect("project");
        ModelScriptFixture::project(root.path(), extension);
        WebCompiler::build_async(root.path(), &BuildOptions::default())
            .await
            .expect("schema build");
        ModelScriptFixture::invoke(root.path()).await;
    }
}

#[tokio::test]
async fn a_script_service_ships_only_the_models_it_names() {
    let root = tempfile::tempdir().expect("project");
    ModelScriptFixture::project(root.path(), "js");
    ModelScriptFixture::write(
        root.path(),
        "client/models/AuditNote.js",
        "/**\n * @typedef {Object} AuditNote\n * @property {string} text\n */\nexport {}\n",
    );
    WebCompiler::build_async(root.path(), &BuildOptions::default())
        .await
        .expect("selective build");
    let service =
        std::fs::read_to_string(root.path().join("dist/.tachyon/services/InputService.js"))
            .expect("service module");
    assert!(service.contains("class ProductInput"), "{service}");
    assert!(!service.contains("AuditNote"), "{service}");
    ModelScriptFixture::invoke(root.path()).await;
}
