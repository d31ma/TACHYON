#![allow(clippy::expect_used)]

use super::{BrowserInteropCheck, NativeWebApis};
use std::path::Path;

#[test]
fn preference_strings_browser_providers_remain_allowed() {
    for extension in ["java", "js", "ts"] {
        let path = format!("client/services/ChoiceService.{extension}");
        BrowserInteropCheck::check_file(Path::new(&path), b"PreferenceStrings preferences;")
            .expect("implemented browser provider");
    }
    assert!(crate::web_api_clients::WebApiClients::is_client_type(
        "PreferenceStrings"
    ));
    assert!(crate::web_api_clients::WebApiClients::is_client_type(
        "Preferences"
    ));
}

#[test]
fn preference_strings_unsupported_browser_languages_have_exact_source_locations() {
    for extension in [
        "rs", "cs", "swift", "kt", "dart", "go", "py", "rb", "lua", "fs",
    ] {
        let path = format!("client/services/nested/ChoiceService.{extension}");
        let source = "  PreferenceStrings preferences;";
        let failure = BrowserInteropCheck::check_file(Path::new(&path), source.as_bytes())
            .expect_err("provider is not implemented for this language");
        let diagnostics = failure.diagnostics();
        assert_eq!(diagnostics.len(), 1, "{extension}");
        let problem = &diagnostics[0];
        assert_eq!(problem.code.to_string(), "TY1317");
        assert!(problem.message.contains("no supported provider"));
        assert!(problem.message.contains("browser language"));
        assert_eq!(problem.spans[0].file, path);
        assert_eq!(problem.spans[0].start, 2);
        assert_eq!(problem.spans[0].end, 2 + "PreferenceStrings".len() as u64);
        assert!(
            problem
                .help
                .as_deref()
                .is_some_and(|help| help.contains("Java browser"))
        );
    }
}

#[test]
fn preference_strings_linked_native_providers_are_refused_on_every_companion() {
    for extension in ["rs", "cs", "swift", "kt"] {
        let path = format!("client/services/nested/ChoiceService.{extension}");
        let failure = NativeWebApis::refuse(Path::new(&path), "  PreferenceStrings preferences;")
            .expect_err("native adapter is not implemented");
        let problem = &failure.diagnostics()[0];
        assert_eq!(problem.code.to_string(), "TY1317");
        assert!(problem.message.contains("linked native platform"));
        assert_eq!(problem.spans[0].file, path);
        assert_eq!(problem.spans[0].start, 2);
    }
}

#[test]
fn preference_strings_inert_text_and_other_preferences_ports_are_unchanged() {
    for extension in ["rs", "cs", "swift", "kt", "dart"] {
        let path = format!("client/services/ChoiceService.{extension}");
        let source = "// PreferenceStrings\nlet text = \"PreferenceStrings\";\nlet MyPreferenceStrings = 1;\nPreferences preferences;";
        BrowserInteropCheck::check_file(Path::new(&path), source.as_bytes())
            .expect("ordinary Preferences has no new refusal");
        NativeWebApis::refuse(
            Path::new(&path),
            "// PreferenceStrings\nlet text = \"PreferenceStrings\";\nlet MyPreferenceStrings = 1;",
        )
        .expect("native comments, strings and longer names remain inert");
    }
    BrowserInteropCheck::check_file(
        Path::new("server/services/ChoiceService.java"),
        b"PreferenceStrings preferences;",
    )
    .expect("browser-specific validation does not change server source policy");
}
