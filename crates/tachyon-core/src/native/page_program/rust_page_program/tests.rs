#![allow(clippy::expect_used)]

use crate::native::registry::NativeCompanionInput;
use crate::project::NativeCompanion;
use std::path::Path;

struct PageFixture;

impl PageFixture {
    const FILES: [(&'static str, &'static str); 7] = [
        (
            "client/setups/PageProgramSetup.rs",
            include_str!(
                "../../../../../../scripts/page-program/rust/client/setups/PageProgramSetup.rs"
            ),
        ),
        (
            "client/remotes/PageProgramClient.rs",
            include_str!(
                "../../../../../../scripts/page-program/rust/client/remotes/PageProgramClient.rs"
            ),
        ),
        (
            "client/stores/ProgramNoteStore.rs",
            include_str!(
                "../../../../../../scripts/page-program/rust/client/stores/ProgramNoteStore.rs"
            ),
        ),
        (
            "client/entities/ProgramNoteEntity.rs",
            include_str!(
                "../../../../../../scripts/page-program/rust/client/entities/ProgramNoteEntity.rs"
            ),
        ),
        (
            "client/models/ProgramState.rs",
            include_str!(
                "../../../../../../scripts/page-program/rust/client/models/ProgramState.rs"
            ),
        ),
        (
            "client/services/ReaderService.rs",
            include_str!(
                "../../../../../../scripts/page-program/rust/client/services/ReaderService.rs"
            ),
        ),
        (
            "client/services/WriterService.rs",
            include_str!(
                "../../../../../../scripts/page-program/rust/client/services/WriterService.rs"
            ),
        ),
    ];
    const SERVICES: [(&'static str, &'static str); 2] =
        [("reader", "ReaderService"), ("writer", "WriterService")];

    fn write(root: &Path) {
        for (path, text) in Self::FILES {
            let text = if path.ends_with("WriterService.rs") {
                text.replace(
                    "use crate::models::ProgramState;",
                    "use crate::models::{ProgramState};",
                )
            } else {
                text.to_owned()
            };
            let file = root.join(path);
            std::fs::create_dir_all(file.parent().expect("parent")).expect("folder");
            std::fs::write(file, text).expect("fixture file");
        }
    }

    fn source() -> String {
        let root = tempfile::tempdir().expect("page fixture");
        Self::write(root.path());
        let inputs = Self::SERVICES
            .iter()
            .map(|(service, class)| NativeCompanionInput {
                snapshot_root: root.path().to_owned(),
                language: NativeCompanion::Rust,
                source: root.path().join(format!("client/services/{class}.rs")),
                service: (*service).to_owned(),
                class: (*class).to_owned(),
                web_assembly: true,
                application_provider: false,
            })
            .collect::<Vec<_>>();
        crate::native::registry::source(&inputs, NativeCompanion::Rust)
            .expect("one Rust page program")
            .expect("emitted Rust page program")
    }
}

#[test]
fn rust_services_of_one_page_share_one_runtime_and_one_setup() {
    let source = PageFixture::source();
    for (once, label) in [
        ("mod TachyonPage {", "one page module"),
        ("pub struct Behaviors;", "one behavior runtime"),
        ("struct PageProgramSetup {", "one Setup"),
        ("use crate::models::ProgramState;", "one merged import"),
    ] {
        assert_eq!(source.matches(once).count(), 1, "{label}: {source}");
    }
    for expected in [
        "\"reader\" => Some(TachyonPage::TachyonService0::tac_create()),",
        "\"writer\" => Some(TachyonPage::TachyonService1::tac_create()),",
        "(\"remember\", TacMember::Task(",
    ] {
        assert!(source.contains(expected), "{expected}: {source}");
    }
}
