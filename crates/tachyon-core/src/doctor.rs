mod command_probe;
mod known_toolchains;
mod service_toolchains;
mod toolchain;
mod toolchain_state;
mod yon_discovery;

pub(crate) use command_probe::CommandProbe;
pub use toolchain::Toolchain;
pub use toolchain_state::ToolchainState;

use crate::Failure;
use crate::failure::{diagnostic, source_span};
use crate::handler::{HandlerSupervisorOptions, RuntimeProbe, RuntimeRequirements};
use crate::template::ServiceRegistry;
use known_toolchains::KnownToolchains;
use serde::Serialize;
use service_toolchains::ServiceToolchains;
use std::fmt::Write as _;
use std::path::Path;
use yon_discovery::YonDiscovery;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DoctorReport {
    pub contract_version: u8,
    pub toolchains: Vec<Toolchain>,
}

impl DoctorReport {
    pub fn check(project_root: impl AsRef<Path>) -> Result<Self, Failure> {
        Self::check_with_options(
            project_root.as_ref(),
            &HandlerSupervisorOptions::from_environment()?,
        )
    }

    #[must_use]
    pub fn is_ready(&self) -> bool {
        !self.toolchains.iter().any(Toolchain::blocks_the_project)
    }

    #[must_use]
    pub fn to_text(&self) -> String {
        self.empty_text().unwrap_or_else(|| self.table_text())
    }

    fn empty_text(&self) -> Option<String> {
        self.toolchains
            .is_empty()
            .then(|| String::from("This project needs no external toolchain.\n"))
    }

    fn table_text(&self) -> String {
        let requirement_width = self.widest(|entry| entry.requirement.len());
        let command_width = self.widest(|entry| entry.command.len());
        let mut text = String::new();
        for entry in &self.toolchains {
            entry.write_line(&mut text, requirement_width, command_width);
        }
        let ready = self
            .toolchains
            .iter()
            .filter(|entry| entry.is_ready())
            .count();
        let _ = writeln!(text, "\n{ready} of {} ready", self.toolchains.len());
        text
    }

    fn widest(&self, width: impl Fn(&Toolchain) -> usize) -> usize {
        self.toolchains.iter().map(width).max().unwrap_or(0)
    }

    fn check_with_options(
        root: &Path,
        options: &HandlerSupervisorOptions,
    ) -> Result<Self, Failure> {
        Self::check_project_exists(root)?;
        let services = ServiceRegistry::discover(root)?;
        let workers = crate::Workers::discover(root)?;
        let yon = YonDiscovery::extensions(root, &workers);
        let mut toolchains = ServiceToolchains::list(root, &services);
        toolchains.extend(crate::tac_python::RuntimeDoctor::for_project(root)?);
        let requirements = RuntimeRequirements::from_extensions(yon.iter().map(String::as_str));
        let resolved = requirements.resolve(&options.runtimes, &options.isolation)?;
        let probes = RuntimeProbe::all_sync(resolved, &options.environment)?;
        toolchains.extend(probes.iter().map(KnownToolchains::runtime));
        Ok(Self {
            contract_version: 1,
            toolchains,
        })
    }

    fn check_project_exists(root: &Path) -> Result<(), Failure> {
        if !root.is_dir() {
            return Err(Self::project_missing(root));
        }
        Ok(())
    }

    fn project_missing(root: &Path) -> Failure {
        Failure::one(diagnostic(
            1601,
            format!("Cannot inspect project '{}'.", root.display()),
            Some(String::from("Point ty doctor at a project directory.")),
            source_span("doctor", 0, 6),
        ))
    }
}

pub fn check(project_root: impl AsRef<Path>) -> Result<DoctorReport, Failure> {
    DoctorReport::check(project_root)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::{DoctorReport, Toolchain, ToolchainState, check};
    use crate::handler::{HandlerRuntimePrograms, HandlerSupervisorOptions, InstalledPrograms};
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    fn project_with(files: &[(&str, &str)]) -> TempDir {
        let root = tempfile::tempdir().expect("project");
        for (relative, contents) in files {
            write_file(root.path(), relative, contents);
        }
        root
    }

    fn write_file(root: &Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        fs::write(path, contents).expect("source");
    }

    fn command_names(report: &DoctorReport) -> Vec<&str> {
        let mut commands = Vec::new();
        for toolchain in &report.toolchains {
            commands.push(toolchain.command.as_str());
        }
        commands
    }

    fn check_with_runtimes(
        root: &Path,
        installed: &[&str],
        configured: Option<&str>,
    ) -> DoctorReport {
        let options = HandlerSupervisorOptions {
            runtimes: InstalledPrograms::runtimes(installed, configured),
            ..HandlerSupervisorOptions::default()
        };
        DoctorReport::check_with_options(root, &options).expect("report")
    }

    fn python_command() -> &'static str {
        if cfg!(windows) { "python" } else { "python3" }
    }

    fn count_of(commands: &[&str], wanted: &str) -> usize {
        commands
            .iter()
            .filter(|command| **command == wanted)
            .count()
    }

    #[test]
    fn a_project_is_told_only_about_what_it_uses() {
        let root = project_with(&[("client/pages/tac.html", "<main>x</main>")]);
        let report = check(root.path()).expect("report");
        assert!(report.toolchains.is_empty(), "{:?}", report.toolchains);
        assert!(report.is_ready());
        assert!(report.to_text().contains("no external toolchain"));
    }

    #[test]
    fn a_native_service_reports_its_native_toolchain() {
        let root = project_with(&[(
            "client/services/CatalogService.rs",
            "#[Service]\nstruct CatalogService;",
        )]);
        let report = check(root.path()).expect("report");
        let entry = report
            .toolchains
            .iter()
            .find(|entry| entry.command == "rustc")
            .expect("rust toolchain reported");
        assert!(
            entry.requirement.contains("macos, windows, linux"),
            "{entry:?}"
        );
        assert!(entry.requirement.contains("services"), "{entry:?}");
        assert!(entry.install.contains("rustup.rs"), "{entry:?}");
        let text = report.to_text();
        assert!(text.contains("macos, windows, linux  rustc"), "{text}");
        match &entry.state {
            ToolchainState::Ready(version) => assert!(!version.is_empty()),
            ToolchainState::Incomplete(reason) => assert!(!reason.is_empty()),
            ToolchainState::Missing => {}
        }
    }

    #[test]
    fn browser_services_report_the_compiler_their_language_needs() {
        let root = project_with(&[
            (
                "client/services/BasketService.ts",
                "@Service\nexport default class BasketService {}\n",
            ),
            (
                "client/services/ClockService.js",
                "@Service\nexport default class ClockService {}\n",
            ),
            (
                "client/services/TallyService.dart",
                "@Service\nclass TallyService {}\n",
            ),
        ]);
        let report = check(root.path()).expect("report");
        let mut requirements = Vec::new();
        for entry in &report.toolchains {
            requirements.push((entry.requirement.as_str(), entry.command.as_str()));
        }
        assert_eq!(
            requirements,
            [
                ("TypeScript services", "tsc"),
                ("Dart browser services", "dart")
            ],
            "{:?}",
            report.toolchains
        );
    }

    #[test]
    fn a_retired_companion_beside_a_view_reports_nothing() {
        let root = project_with(&[
            ("client/pages/tac.html", "<div>x</div>"),
            ("client/pages/tac.rs", "fn main() {}"),
        ]);
        let report = check(root.path()).expect("report");
        assert!(report.toolchains.is_empty(), "{:?}", report.toolchains);
    }

    #[test]
    fn yon_sources_report_only_the_runtime_toolchains_the_project_uses() {
        let root = project_with(&[
            ("server/controllers/AController.py", "source"),
            ("server/controllers/BController.java", "source"),
        ]);
        let report = check(root.path()).expect("report");
        let commands = command_names(&report);
        assert!(commands.contains(&python_command()), "{commands:?}");
        assert!(commands.contains(&"java"), "{commands:?}");
        assert!(!commands.contains(&"bun"), "{commands:?}");
    }

    #[test]
    fn yon_runtime_checks_use_resolved_overrides_without_disclosing_them() {
        let root = project_with(&[(
            "server/controllers/ExampleController.js",
            "export class Handler {}",
        )]);
        let canary = "/private/credentials/doctor-runtime-canary";
        let options = HandlerSupervisorOptions {
            runtimes: HandlerRuntimePrograms {
                javascript: PathBuf::from(canary),
                ..HandlerRuntimePrograms::from_lookup(|_| None)
            },
            ..HandlerSupervisorOptions::default()
        };
        let report = DoctorReport::check_with_options(root.path(), &options).expect("report");
        let javascript = report.toolchains.first().expect("javascript");
        assert_eq!(javascript.command, "YON_JAVASCRIPT_RUNTIME");
        assert_eq!(javascript.state, ToolchainState::Missing);
        let human = report.to_text();
        let json = serde_json::to_string(&report).expect("json");
        assert!(!human.contains(canary), "{human}");
        assert!(!json.contains(canary), "{json}");
        assert!(human.contains("YON_JAVASCRIPT_RUNTIME"), "{human}");
    }

    #[test]
    fn yon_discovery_is_limited_to_maintained_source_roots_and_configured_workers() {
        let root = project_with(&[
            ("server/controllers/AController.js", "source"),
            ("server/controllers/BController.py", "source"),
            ("server/controllers/typescript/CController.ts", "source"),
            ("server/controllers/DController.java", "source"),
            ("server/controllers/EController.cs", "source"),
            ("server/controllers/FController.kt", "source"),
            ("server/controllers/HController.groovy", "source"),
            ("server/services/GService.swift", "source"),
            ("middleware.php", "source"),
            ("server/workers/cleanup.rs", "source"),
            ("fixtures/decoy/yon.ts", "source"),
            ("docs/examples/yon.py", "source"),
            ("generated/preludes/yon.php", "source"),
            (
                ".tachyonrc",
                r#"{"workers":{"server/workers/cleanup.rs":{"every_seconds":60}}}"#,
            ),
        ]);
        let report = check_with_runtimes(root.path(), &["node"], None);
        let commands = command_names(&report);
        for expected in [
            "node", "java", "dotnet", "kotlinc", "groovyc", "php-cgi", "cargo",
        ] {
            assert!(commands.contains(&expected), "{expected}: {commands:?}");
        }
        assert!(commands.contains(&python_command()), "{commands:?}");
        assert_eq!(count_of(&commands, "php-cgi"), 1);
        assert_eq!(count_of(&commands, "node"), 1);
        assert_eq!(count_of(&commands, "java"), 1);
    }

    #[test]
    fn yon_javascript_runtime_is_the_installed_program_or_the_configured_one() {
        let root = project_with(&[("server/controllers/AController.js", "source")]);
        let cases: [(&[&str], Option<&str>, &str); 6] = [
            (&["node"], None, "node"),
            (&["bun"], None, "bun"),
            (&["bun", "node"], None, "node"),
            (&[], None, "node"),
            (&["node"], Some("bun"), "bun"),
            (
                &["bun", "node"],
                Some("/opt/javascript/run"),
                "YON_JAVASCRIPT_RUNTIME",
            ),
        ];
        for (installed, configured, expected) in cases {
            let report = check_with_runtimes(root.path(), installed, configured);
            let commands = command_names(&report);
            assert_eq!(commands, [expected], "{installed:?} {configured:?}");
        }
    }

    #[test]
    fn fsharp_and_csharp_handlers_report_dotnet_once() {
        let root = project_with(&[
            ("server/controllers/AController.fs", "source"),
            ("server/controllers/BController.cs", "source"),
        ]);
        let report = check(root.path()).expect("report");
        let commands = command_names(&report);
        assert_eq!(count_of(&commands, "dotnet"), 1, "{commands:?}");
    }

    #[test]
    fn go_handlers_report_the_go_compiler_once() {
        let root = project_with(&[
            ("server/controllers/AController.go", "source"),
            ("middleware.go", "source"),
            ("server/workers/cleanup.go", "source"),
            (
                ".tachyonrc",
                r#"{"workers":{"server/workers/cleanup.go":{"every_seconds":60}}}"#,
            ),
        ]);
        let report = check(root.path()).expect("report");
        let commands = command_names(&report);
        assert_eq!(commands, ["go"], "{:?}", report.toolchains);
        let entry = &report.toolchains[0];
        assert_eq!(entry.requirement, "Yon Go compiler 'go'");
        assert_eq!(
            entry.install,
            "Install Go 1.27 or newer before starting Tachyon."
        );
    }

    #[test]
    fn ruby_handlers_report_the_ruby_runtime_once() {
        let root = project_with(&[
            ("server/controllers/AController.rb", "source"),
            ("middleware.rb", "source"),
            ("server/workers/cleanup.rb", "source"),
            (
                ".tachyonrc",
                r#"{"workers":{"server/workers/cleanup.rb":{"every_seconds":60}}}"#,
            ),
        ]);
        let report = check(root.path()).expect("report");
        let commands = command_names(&report);
        assert_eq!(commands, ["ruby"], "{:?}", report.toolchains);
        let entry = &report.toolchains[0];
        assert_eq!(entry.requirement, "Yon Ruby runtime 'ruby'");
        assert_eq!(
            entry.install,
            "Install Ruby 3.3 or newer, or correct YON_RUBY_RUNTIME."
        );
    }

    #[test]
    fn lua_handlers_report_the_lua_runtime_once() {
        let root = project_with(&[
            ("server/controllers/AController.lua", "source"),
            ("middleware.lua", "source"),
        ]);
        let report = check(root.path()).expect("report");
        let commands = command_names(&report);
        assert_eq!(commands, ["lua"], "{:?}", report.toolchains);
        let entry = &report.toolchains[0];
        assert_eq!(entry.requirement, "Yon Lua runtime 'lua'");
        assert_eq!(
            entry.install,
            "Install Lua 5.4 or newer and put its `lua` program on PATH before starting Tachyon."
        );
    }

    #[test]
    fn a_lua_web_service_reports_the_tools_that_build_and_test_it() {
        let root = project_with(&[(
            "client/services/BasketService.lua",
            "-- @Service\n---@class BasketService\nBasketService = {\n  ---@type integer\n  count = 0,\n}\n",
        )]);
        let report = check(root.path()).expect("report");
        let commands = command_names(&report);
        let built = crate::tac_lua::LuaRuntimeBuild::cached_path().is_file();
        let first = ["emcc", "tachyon-lua.wasm"][usize::from(built)];
        assert!(commands[0].starts_with(first), "{commands:?}");
        assert!(commands.contains(&"lua"), "{commands:?}");
        for entry in &report.toolchains {
            assert_eq!(entry.requirement, "Lua services on the web");
            assert!(entry.required, "{entry:?}");
        }
    }

    #[test]
    fn a_ruby_web_service_reports_the_tools_that_build_its_runtime() {
        let root = project_with(&[(
            "client/services/BasketService.rb",
            "#[Service]\nclass BasketService\n  attr_reader :count\nend\n",
        )]);
        let report = check(root.path()).expect("report");
        let commands = command_names(&report);
        if crate::tac_ruby::RubyRuntimeBuild::cached_path().is_file() {
            assert_eq!(commands, ["tachyon-ruby.wasm"], "{:?}", report.toolchains);
        } else {
            assert!(
                commands.iter().any(|command| command.starts_with("emcc")),
                "{commands:?}"
            );
            assert!(commands.contains(&"ruby"), "{commands:?}");
        }
        for entry in &report.toolchains {
            assert_eq!(entry.requirement, "Ruby services on the web");
            assert!(entry.required, "{entry:?}");
        }
    }

    #[test]
    fn a_go_web_service_reports_the_go_toolchain() {
        let root = project_with(&[(
            "client/services/BasketService.go",
            "// @Service\ntype BasketService struct{ Count int }\n",
        )]);
        let report = check(root.path()).expect("report");
        assert_eq!(
            command_names(&report),
            ["go", "tinygo", "wasm-opt"],
            "{:?}",
            report.toolchains
        );
        assert!(!report.toolchains[1].required, "{:?}", report.toolchains);
        assert!(
            report.toolchains[1]
                .install
                .contains("standard Go compiler")
        );
        assert!(!report.toolchains[2].required, "{:?}", report.toolchains);
        let entry = &report.toolchains[0];
        assert_eq!(entry.requirement, "Go services on the web");
        assert!(entry.install.contains("Go 1.27 or newer"), "{entry:?}");
        if let ToolchainState::Ready(version) = &entry.state {
            assert!(
                crate::handler::RuntimeProbe::supports_go_version(version),
                "{version}"
            );
        }
    }

    #[test]
    fn a_java_web_service_reports_teavm_and_binaryen() {
        let root = project_with(&[(
            "client/services/BasketService.java",
            "@Service\nclass BasketService {\n    int count = 0;\n}\n",
        )]);
        let report = check(root.path()).expect("report");
        assert_eq!(
            command_names(&report),
            ["javac", "teavm", "wasm-opt"],
            "{:?}",
            report.toolchains
        );
        let compiler = &report.toolchains[0];
        assert_eq!(compiler.requirement, "Java services on the web");
        assert!(compiler.install.contains("JDK 21"), "{compiler:?}");
        let tea_vm = &report.toolchains[1];
        assert_eq!(tea_vm.requirement, "Java services on the web");
        assert!(tea_vm.required, "{tea_vm:?}");
        assert!(tea_vm.install.contains("ty doctor --install"), "{tea_vm:?}");
        assert!(tea_vm.install.contains("TeaVM 0.15.0"), "{tea_vm:?}");
    }

    #[cfg(unix)]
    #[test]
    fn yon_discovery_never_follows_outside_or_cyclic_directory_symlinks() {
        use std::os::unix::fs::symlink;

        let root = project_with(&[("server/controllers/LiveController.js", "source")]);
        let outside = project_with(&[("nested/yon.py", "source")]);
        let controllers = root.path().join("server/controllers");
        symlink(outside.path(), controllers.join("outside")).expect("outside link");
        std::fs::create_dir_all(controllers.join("live")).expect("live folder");
        symlink(&controllers, controllers.join("live/parent")).expect("cycle");
        let report = check_with_runtimes(root.path(), &["node"], None);
        let commands = command_names(&report);
        assert!(commands.contains(&"node"), "{commands:?}");
        assert!(!commands.contains(&python_command()), "{commands:?}");
    }

    fn toolchain(state: ToolchainState, required: bool) -> Toolchain {
        Toolchain {
            requirement: String::from("Kotlin services for android"),
            command: String::from("kotlinc"),
            state,
            install: String::from("Install Kotlin"),
            required,
        }
    }

    fn report_of(toolchains: Vec<Toolchain>) -> DoctorReport {
        DoctorReport {
            contract_version: 1,
            toolchains,
        }
    }

    #[test]
    fn a_missing_or_old_required_toolchain_makes_the_project_not_ready() {
        let missing = report_of(vec![toolchain(ToolchainState::Missing, true)]);
        assert!(!missing.is_ready());
        assert!(
            missing.to_text().starts_with("missing "),
            "{}",
            missing.to_text()
        );
        let old = ToolchainState::Incomplete(String::from("go1.26"));
        let too_old = report_of(vec![toolchain(old, true)]);
        assert!(!too_old.is_ready());
        assert!(
            too_old.to_text().starts_with("partial "),
            "{}",
            too_old.to_text()
        );
    }

    #[test]
    fn a_missing_optional_toolchain_is_information_only() {
        let report = report_of(vec![
            toolchain(ToolchainState::Missing, false),
            toolchain(ToolchainState::Ready(String::from("2.1.10")), true),
        ]);
        assert!(report.is_ready());
        let text = report.to_text();
        assert!(text.starts_with("info "), "{text}");
        assert!(text.contains("\nready "), "{text}");
    }

    #[test]
    fn native_target_toolchains_are_optional_and_the_others_are_required() {
        let root = project_with(&[
            (
                "client/services/CatalogService.kt",
                "@Service\nclass CatalogService\n",
            ),
            (
                "client/services/CatalogService.js",
                "@Service\nexport default class CatalogService {}\n",
            ),
            ("server/controllers/AController.py", "source"),
        ]);
        let report = check(root.path()).expect("report");
        let mut required = Vec::new();
        for entry in &report.toolchains {
            required.push((entry.command.as_str(), entry.required));
        }
        assert_eq!(
            required,
            [("kotlinc", false), (python_command(), true)],
            "{:?}",
            report.toolchains
        );
    }

    #[test]
    fn a_missing_project_fails_closed() {
        let error = check("/definitely/not/a/project").expect_err("missing");
        assert!(error.to_string().contains("TY1601"), "{error}");
    }
}
