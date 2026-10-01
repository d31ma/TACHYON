#![allow(clippy::expect_used)]

use super::{BrowserInteropCheck, NativeWebApis};
use std::path::Path;

#[test]
fn document_title_provider_is_located_for_unsupported_browser_languages() {
    for extension in [
        "java", "rs", "cs", "swift", "dart", "go", "py", "rb", "lua", "fs",
    ] {
        let path = format!("client/services/TitleService.{extension}");
        let failure = BrowserInteropCheck::check_file(Path::new(&path), b"  DocumentTitle port;")
            .expect_err("missing provider");
        let problem = &failure.diagnostics()[0];
        assert!(problem.message.contains("DocumentTitle"));
        assert_eq!(problem.spans[0].start, 2);
        assert_eq!(problem.spans[0].end, 15);
    }
}

#[test]
fn document_title_provider_accepts_kotlin_and_script_page_contracts() {
    for extension in ["kt", "js", "ts"] {
        let path = format!("client/services/TitleService.{extension}");
        BrowserInteropCheck::check_file(Path::new(&path), b"DocumentTitle port;")
            .expect("browser provider");
    }
}

#[test]
fn document_title_provider_refuses_linked_native_and_ignores_inert_words() {
    for extension in ["rs", "cs", "swift", "kt"] {
        let path = format!("client/services/TitleService.{extension}");
        let failure = NativeWebApis::refuse_preferences(Path::new(&path), "DocumentTitle port;")
            .expect_err("no native page provider");
        assert!(failure.diagnostics()[0].message.contains("linked native"));
    }
    BrowserInteropCheck::check_file(
        Path::new("client/services/TitleService.java"),
        b"// DocumentTitle\nString text = \"DocumentTitle\"; MyDocumentTitle port;",
    )
    .expect("inert words");
}
