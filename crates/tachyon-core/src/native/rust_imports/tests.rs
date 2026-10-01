#![allow(clippy::expect_used)]

use super::RustImports;
use std::path::Path;

#[test]
fn rust_utility_import_scopes_preserve_member_modifiers_and_inert_text() {
    let source = r#"
use std::cmp::Ordering;
struct TitleUtils;
impl TitleUtils {
    pub(crate) const TITLE: &'static str = "fn fake() { use std::sync::atomic::Ordering; }";
    const fn number() -> i64 { 1 }
    pub fn compare() -> Ordering { Ordering::Equal }
    fn private() -> i64 { Self::number() }
}
impl Default for TitleUtils { fn default() -> Self { Self } }
"#;
    let scoped = RustImports::scoped(
        Path::new("client/utils/TitleUtils.rs"),
        source,
        "TitleUtils",
    )
    .expect("local imports");
    assert!(scoped.contains("mod __TachyonUtility_TitleUtils"));
    assert!(scoped.contains("pub(super) struct TitleUtils"));
    assert!(scoped.contains("pub(crate) const TITLE"));
    assert!(scoped.contains("pub(super) const fn number"));
    assert!(scoped.contains("pub fn compare"));
    assert!(scoped.contains("pub(super) fn private"));
    assert!(scoped.contains("impl Default for TitleUtils { fn default() -> Self"));
    assert!(scoped.contains("\"fn fake() { use std::sync::atomic::Ordering; }\""));
    assert!(scoped.contains("use self::__TachyonUtility_TitleUtils::TitleUtils;"));
}

#[test]
fn rust_utility_import_scopes_reject_conflicting_bindings_within_one_file() {
    let source = "use std::cmp::Ordering;\nuse std::sync::atomic::Ordering;\nstruct TitleUtils;\n";
    let error = RustImports::scoped(
        Path::new("client/utils/TitleUtils.rs"),
        source,
        "TitleUtils",
    )
    .expect_err("same-file conflict");
    assert!(error.to_string().contains("TY1406"), "{error}");
}

#[tokio::test]
async fn rust_utility_import_scopes_preserve_public_fixture_reexports() {
    let root = tempfile::tempdir().expect("fixture");
    let scoped = RustImports::scoped(
        Path::new("client/utils/TitleUtils.rs"),
        r#"
use std::cmp::Ordering;
pub struct TitleUtils;
impl TitleUtils {
    pub const TITLE: &'static str = "ready";
    pub fn compare(left: i64, right: i64) -> Ordering { Self::private(left, right) }
    fn private(left: i64, right: i64) -> Ordering { left.cmp(&right) }
}
"#,
        "TitleUtils",
    )
    .expect("public utility");
    assert!(scoped.contains("pub struct TitleUtils"));
    assert!(scoped.contains("pub const TITLE"));
    assert!(scoped.contains("pub fn compare"));
    assert!(scoped.contains("pub(super) fn private"));
    let source = root.path().join("public-reexport.rs");
    std::fs::write(&source, format!("mod canonical {{ {scoped} }}\n")).expect("source");
    let caller = r#"
pub mod fixture_utils { pub use crate::canonical::TitleUtils; }
fn main() {
    assert_eq!(fixture_utils::TitleUtils::TITLE, "ready");
    assert_eq!(fixture_utils::TitleUtils::compare(1, 2), std::cmp::Ordering::Less);
}
"#;
    super::super::native_page_graph_tests::NativePageGraphCheck::execute(&source, caller).await;
}
