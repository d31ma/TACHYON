#![allow(clippy::expect_used)]

use super::RustLocalPaths;
use std::collections::BTreeSet;
use std::path::Path;

#[test]
fn rust_utility_boundary_maps_only_validated_names_and_code_references() {
    let names = BTreeSet::from([String::from("TitleUtils")]);
    let paths = RustLocalPaths::read(
        Path::new("LabelService.rs"),
        "struct TitleUtils;",
        "",
        &names,
    )
    .expect("paths");
    let source = "fn text() { let literal = \"crate::utils::TitleUtils\"; /* crate::utils::TitleUtils */ crate::utils::TitleUtils::text(); crate::utils::ForeignUtils::text(); }";
    let mapped = paths.source(source).expect("mapped code");
    assert!(mapped.contains("crate::TachyonRustServices::TitleUtils::text()"));
    assert!(mapped.contains("\"crate::utils::TitleUtils\""));
    assert!(mapped.contains("/* crate::utils::TitleUtils */"));
    assert!(mapped.contains("crate::utils::ForeignUtils::text()"));
    assert_eq!(
        paths.import("use crate::utils::TitleUtils as Title;"),
        Some(String::from("use self::TitleUtils as Title;"))
    );
    assert_eq!(
        paths.import("use crate::utils::ForeignUtils;"),
        Some(String::from("use crate::utils::ForeignUtils;"))
    );
    assert_eq!(
        paths.import("use crate::utils as tools;"),
        Some(String::from("use crate::utils as tools;"))
    );
}
