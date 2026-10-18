#![allow(clippy::expect_used)]

use super::config::TacMember;
use super::target_support::TargetSupport;
use crate::AppTarget;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::PathBuf;
use tachyon_contracts::NativeTarget;
use tachyon_diagnostics::DiagnosticCode;

struct TargetSupportTests;

impl TargetSupportTests {
    const TARGETS: [AppTarget; 6] = [
        AppTarget::Web,
        AppTarget::Macos,
        AppTarget::Windows,
        AppTarget::Linux,
        AppTarget::Ios,
        AppTarget::Android,
    ];
    const TAB_FOLDER: &'static str =
        "../../website/content/docs/application-configuration/manifest-members-per-target";
    const WRITE_TABS: &'static str = "TACHYON_WRITE_TARGET_SUPPORT";

    fn project(manifest: &Value) -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("support project");
        std::fs::create_dir_all(root.path().join("client")).expect("client directory");
        std::fs::write(
            root.path().join("client/manifest.json"),
            manifest.to_string(),
        )
        .expect("manifest");
        root
    }

    fn notes(manifest: &Value, target: AppTarget) -> Vec<String> {
        let root = Self::project(manifest);
        TargetSupport::notes(root.path(), target).expect("notes")
    }

    fn tab_path(target: AppTarget) -> PathBuf {
        let name = TargetSupport::platform(target).to_lowercase();
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(Self::TAB_FOLDER)
            .join(format!("{name}.txt"))
    }
}

#[test]
fn every_framework_member_states_its_reach_on_all_six_targets() {
    let table = TargetSupport::load().expect("table");
    let paths: BTreeSet<&str> = table
        .entries()
        .iter()
        .flat_map(|entry| entry.paths().iter().map(String::as_str))
        .collect();
    for member in TacMember::FRAMEWORK_MEMBERS {
        let path = format!("tac.{member}");
        assert!(
            paths
                .iter()
                .any(|declared| declared == &path || declared.starts_with(&format!("{path}."))),
            "{path} has no entry in target_support.json"
        );
    }
}

#[test]
fn every_entry_has_a_unique_member_and_real_codes_and_text() {
    let table = TargetSupport::load().expect("table");
    let mut members = BTreeSet::new();
    for entry in table.entries() {
        assert!(members.insert(entry.member()), "{} repeats", entry.member());
        for target in TargetSupportTests::TARGETS {
            let reach = entry.reach(target);
            assert!(
                !reach.text().trim().is_empty(),
                "{} is empty",
                entry.member()
            );
            if let Some((code, _)) = reach.refusal() {
                assert!(DiagnosticCode::from_number(code).is_some(), "TY{code}");
            }
        }
    }
}

#[test]
fn a_build_prints_nothing_for_members_the_developer_did_not_declare() {
    let manifest = json!({"name": "Plain"});
    for target in TargetSupportTests::TARGETS {
        let notes = TargetSupportTests::notes(&manifest, target);
        assert!(
            notes.iter().all(|note| !note.contains("theme_color")),
            "{notes:?}"
        );
    }
    let root = tempfile::tempdir().expect("empty project");
    assert!(
        TargetSupport::notes(root.path(), AppTarget::Ios)
            .expect("no manifest")
            .is_empty()
    );
}

#[test]
fn an_integrated_title_bar_gets_a_note_everywhere_but_macos() {
    let manifest = json!({"tac": {"application": {"window": {"titleBar": "integrated"}}}});
    let mac = TargetSupportTests::notes(&manifest, AppTarget::Macos);
    assert!(mac.iter().all(|note| !note.contains("titleBar")), "{mac:?}");
    for target in [
        AppTarget::Windows,
        AppTarget::Linux,
        AppTarget::Ios,
        AppTarget::Android,
    ] {
        let notes = TargetSupportTests::notes(&manifest, target);
        let platform = TargetSupport::platform(target);
        assert!(
            notes.iter().any(
                |note| note.starts_with("Note: tac.application.window.titleBar")
                    && note.contains(&format!("The {platform} app keeps the standard"))
            ),
            "{notes:?}"
        );
    }
}

#[test]
fn a_web_build_names_tac_application_once() {
    let manifest = json!({"theme_color": "#123456", "tac": {"application": {
        "id": "ma.del.example", "version": "1.2.0",
        "window": {"width": 800, "height": 600, "controls": ["close"]}
    }}});
    let notes = TargetSupportTests::notes(&manifest, AppTarget::Web);
    assert_eq!(
        notes,
        vec![String::from(
            "Note: tac.application is read only by native builds. A web build checks its shape and then ignores it."
        )]
    );
    let windows = TargetSupportTests::notes(&manifest, AppTarget::Windows);
    assert!(!windows.iter().any(|note| note.contains("width and height")));
    assert!(windows.iter().any(|note| note.contains("theme_color")));
    let linux = TargetSupportTests::notes(&manifest, AppTarget::Linux);
    assert!(!linux.iter().any(|note| note.contains("width and height")));
    assert!(linux.iter().any(|note| note.contains("GTK header bar")));
}

#[test]
fn a_listed_browser_api_is_declared_by_its_name() {
    let manifest = json!({"tac": {"browserApis": ["geolocation"]}});
    let android = TargetSupportTests::notes(&manifest, AppTarget::Android);
    assert!(android.iter().any(|note| note.contains("geolocation")));
    assert!(android.iter().all(|note| !note.contains("clipboard")));
}

#[test]
fn helper_programs_are_refused_on_mobile_with_the_platform_rule() {
    let manifest =
        json!({"tac": {"application": {"rust": {"executables": {"convert": "Convert.rs"}}}}});
    let root = TargetSupportTests::project(&manifest);
    for target in [NativeTarget::Ios, NativeTarget::Android] {
        let failure =
            TargetSupport::native_notes(root.path(), target).expect_err("mobile refuses helpers");
        let diagnostic = &failure.diagnostics()[0];
        assert_eq!(diagnostic.code.number(), 1601);
        assert!(
            diagnostic
                .message
                .contains("tac.application.rust.executables")
        );
        assert!(
            diagnostic
                .help
                .as_deref()
                .is_some_and(|help| help.contains("may not start other programs"))
        );
    }
    assert!(TargetSupport::native_notes(root.path(), NativeTarget::Linux).is_ok());
    let empty = json!({"tac": {"application": {"menus": {}, "rust": {"executables": {}}}}});
    let root = TargetSupportTests::project(&empty);
    assert!(TargetSupport::native_notes(root.path(), NativeTarget::Ios).is_ok());
}

#[test]
fn explain_marks_the_declared_members_for_each_target() {
    let manifest = json!({"theme_color": "#000000"});
    let root = TargetSupportTests::project(&manifest);
    let text =
        TargetSupport::explain(root.path(), &[AppTarget::Ios, AppTarget::Web]).expect("explain");
    assert!(text.starts_with("iOS\n"));
    assert!(text.contains("\nWeb\n"));
    assert!(text.contains("theme_color (declared)\n  adapted: "));
    assert!(text.contains("tac.application.menus\n  refused with TY1601: "));
}

#[test]
fn the_website_platform_tabs_match_the_table() {
    let table = TargetSupport::load().expect("table");
    let write = std::env::var_os(TargetSupportTests::WRITE_TABS).is_some();
    for target in TargetSupportTests::TARGETS {
        let expected = table.target_text(target, &Value::Null);
        let path = TargetSupportTests::tab_path(target);
        if write {
            std::fs::write(&path, &expected).expect("write tab");
        }
        let actual = std::fs::read_to_string(&path).unwrap_or_default();
        assert_eq!(
            actual,
            expected,
            "{} is stale; run {}=1 cargo test -p tachyon-core --lib target_support",
            path.display(),
            TargetSupportTests::WRITE_TABS
        );
    }
}
