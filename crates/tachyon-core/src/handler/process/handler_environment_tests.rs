#![allow(clippy::expect_used)]

use super::{AdapterFiles, HandlerLanguage, HandlerSource, HandlerSupervisor};
use crate::HandlerSupervisorOptions;
use std::collections::BTreeSet;
use std::path::Path;

struct LanguageSources;

impl LanguageSources {
    const CONFIGURATION: &'static str = r#"{"server":{"environment":["CARGO_PKG_*"]}}"#;
    const PASSED_NAME: &'static str = "CARGO_PKG_NAME";
    const HELD_BACK_NAME: &'static str = "CARGO_MANIFEST_DIR";
    const INTERPRETED: [(&'static str, &'static str); 4] = [
        (
            "server/workers/environment.js",
            "export class Handler { static GET() { return { ok: true } } }\n",
        ),
        (
            "server/workers/environment.ts",
            "export class Handler { static GET() { return { ok: true } } }\n",
        ),
        (
            "server/workers/environment.py",
            "class Handler:\n    @staticmethod\n    def GET(request):\n        return {\"ok\": True}\n",
        ),
        (
            "server/workers/environment.rb",
            "class Handler\n  def self.GET(request)\n    { \"ok\" => true }\n  end\nend\n",
        ),
    ];
    const COMPILED: (&'static str, &'static str) = (
        "server/workers/environment.go",
        "package main\n\ntype Handler struct{}\n\nfunc (Handler) GET() any {\n\treturn map[string]any{\"ok\": true}\n}\n",
    );

    fn write(root: &Path, relative: &str, text: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(path, text).expect("source");
    }

    fn go_usable() -> bool {
        let found = std::process::Command::new("go")
            .arg("version")
            .output()
            .is_ok_and(|output| output.status.success());
        let required = cfg!(unix) && std::env::var_os("CI").is_some();
        assert!(found || !required, "go is required in CI");
        found
    }

    fn passed_names(root: &Path, relative: &str) -> (HandlerLanguage, BTreeSet<String>) {
        let (source, application) =
            HandlerSource::discover_with_application(root, relative).expect("handler source");
        let supervisor = HandlerSupervisor::new(HandlerSupervisorOptions {
            application,
            ..HandlerSupervisorOptions::default()
        })
        .expect("supervisor");
        let adapter = AdapterFiles::materialize(source.language()).expect("adapter");
        let planned = supervisor.handler_command(&source, &adapter);
        let names = planned
            .command
            .as_std()
            .get_envs()
            .filter(|(_, value)| value.is_some())
            .map(|(name, _)| name.to_string_lossy().into_owned())
            .collect();
        (source.language(), names)
    }

    const fn language_path(language: HandlerLanguage) -> &'static str {
        match language {
            HandlerLanguage::JavaScript => "javascript adapter",
            HandlerLanguage::TypeScript => "typescript adapter",
            HandlerLanguage::Python => "python adapter",
            HandlerLanguage::Ruby => "ruby adapter",
            HandlerLanguage::Direct => "compiled or prepared program",
        }
    }
}

#[test]
fn every_language_path_passes_configured_variables_and_holds_back_the_rest() {
    assert!(
        std::env::var_os(LanguageSources::PASSED_NAME).is_some()
            && std::env::var_os(LanguageSources::HELD_BACK_NAME).is_some(),
        "cargo sets both variables for its test processes"
    );
    let root = tempfile::tempdir().expect("project");
    LanguageSources::write(
        root.path(),
        "server/application.json",
        LanguageSources::CONFIGURATION,
    );
    let mut sources = LanguageSources::INTERPRETED.to_vec();
    if LanguageSources::go_usable() {
        sources.push(LanguageSources::COMPILED);
    }
    let mut covered = BTreeSet::new();
    for (relative, text) in sources {
        LanguageSources::write(root.path(), relative, text);
        let (language, names) = LanguageSources::passed_names(root.path(), relative);
        let path = LanguageSources::language_path(language);
        assert!(names.contains(LanguageSources::PASSED_NAME), "{path}");
        assert!(!names.contains(LanguageSources::HELD_BACK_NAME), "{path}");
        assert!(names.contains("YON_HANDLER_PROTOCOL"), "{path}");
        covered.insert(path);
    }
    assert!(covered.len() >= LanguageSources::INTERPRETED.len());
}
