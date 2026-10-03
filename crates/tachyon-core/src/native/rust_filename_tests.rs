#![allow(clippy::expect_used)]

use super::native_page_graph_tests::NativePageGraphCheck;
use super::{NativeBuildOptions, NativeCompiler};
use std::path::Path;
use tachyon_contracts::NativeTarget;

struct RustFilenameCheck;

impl RustFilenameCheck {
    const SERVICE: &str = "use crate::utils::TitleUtils as Titles;\nuse crate::models::Guide;\n#[Service]\n#[derive(Default)]\nstruct LabelService;\nimpl LabelService { pub fn read(&self) -> String { let guide = Guide { title: Titles::text() }; guide.title } }\n";
    const UTILITY: &str = "pub struct TitleUtils;\nimpl TitleUtils { const TITLE: &'static str = \"Ready\"; pub fn text() -> String { Self::TITLE.to_owned() } }\n";
    const MODEL: &str = "pub struct Guide { pub title: String }\n";
    const CALLER: &str = r##"
fn main() {
    let owner = "00000000-0000-4000-8000-000000000001";
    let init = tac_handle(&format!(r#"{{"route":"/","service":"label","owner":"{owner}","op":"init"}}"#));
    assert!(!init.contains("error"), "{init}");
    let reply = tac_handle(&format!(r#"{{"route":"/","service":"label","owner":"{owner}","op":"call","name":"read","args":[]}}"#));
    assert_eq!(reply, r#"{"value":"Ready"}"#);
    let _ = tac_handle("{\"op\":\"host-dispose\"}");
}
"##;

    fn write(root: &Path, path: &str, source: &str) {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        std::fs::write(path, source).expect("fixture");
    }

    async fn execute(names: [&str; 3]) {
        let root = tempfile::tempdir().expect("project");
        Self::write(root.path(), "client/pages/tac.html", "<label-card />");
        Self::write(
            root.path(),
            "client/components/label-card/tac.html",
            "<p>{label.read()}</p>",
        );
        Self::write(
            root.path(),
            &format!("client/services/feature/native/{}.rs", names[0]),
            Self::SERVICE,
        );
        Self::write(
            root.path(),
            &format!("client/shared/utils/feature/{}.rs", names[1]),
            Self::UTILITY,
        );
        Self::write(
            root.path(),
            &format!("client/models/feature/{}.rs", names[2]),
            Self::MODEL,
        );
        let options = NativeBuildOptions {
            output_directory: "dist".into(),
            target: NativeTarget::Macos,
            package: false,
        };
        let built = NativeCompiler::build_async(root.path(), &options)
            .await
            .expect("ordinary native collection and staging");
        let source = built.output_directory().join("project/companion.rs");
        NativePageGraphCheck::execute(&source, Self::CALLER).await;
    }
}

#[tokio::test]
async fn rust_snake_case_nested_roles_preserve_native_imports_and_catalog_identity() {
    RustFilenameCheck::execute(["label_service", "title_utils", "guide"]).await;
    RustFilenameCheck::execute(["LabelService", "TitleUtils", "Guide"]).await;
}
