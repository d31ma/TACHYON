mod dotnet_version_outcome;
mod prepared_probe;
mod probe_kind;
mod runtime_probe;
mod runtime_probe_result;
mod runtime_probe_state;
mod runtime_requirement;
mod runtime_requirement_key;
mod yon_language;

pub(crate) use runtime_probe::RuntimeProbe;
pub(crate) use runtime_probe_result::RuntimeProbeResult;
pub(crate) use runtime_probe_state::RuntimeProbeState;
pub(crate) use runtime_requirement::RuntimeRequirement;
pub(crate) use runtime_requirement_key::RuntimeRequirementKey;
pub(crate) use yon_language::YonLanguage;

use super::{FirecrackerIsolation, HandlerRuntimePrograms, HandlerSource, YonIsolationPolicy};
use crate::Failure;
use crate::failure::diagnostic;
use dotnet_version_outcome::DotnetVersionOutcome;
use prepared_probe::PreparedProbe;
use probe_kind::ProbeKind;
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct RuntimeRequirements {
    languages: BTreeSet<YonLanguage>,
    gateway_programs: BTreeSet<String>,
    uses_gateways: bool,
}

impl RuntimeRequirements {
    pub(crate) fn from_sources<'a>(sources: impl IntoIterator<Item = &'a HandlerSource>) -> Self {
        let mut requirements = Self::default();
        sources
            .into_iter()
            .for_each(|source| requirements.add_source(source));
        requirements
    }

    pub(crate) fn from_gateways<'a>(sources: impl IntoIterator<Item = &'a HandlerSource>) -> Self {
        let mut requirements = Self::default();
        sources
            .into_iter()
            .for_each(|source| requirements.add_gateways(source));
        requirements
    }

    pub(crate) fn from_extensions<'a>(extensions: impl IntoIterator<Item = &'a str>) -> Self {
        Self {
            languages: extensions
                .into_iter()
                .filter_map(YonLanguage::from_extension)
                .collect(),
            ..Self::default()
        }
    }

    pub(crate) fn resolve(
        &self,
        runtimes: &HandlerRuntimePrograms,
        isolation: &YonIsolationPolicy,
    ) -> Result<Vec<RuntimeRequirement>, Failure> {
        isolation.firecracker().map_or_else(
            || Ok(self.process_requirements(runtimes)),
            |policy| self.firecracker_requirements(policy, isolation),
        )
    }

    fn add_source(&mut self, source: &HandlerSource) {
        self.languages
            .extend(YonLanguage::from_path(Path::new(source.relative_path())));
        self.add_gateways(source);
    }

    fn add_gateways(&mut self, source: &HandlerSource) {
        if let Some(gateways) = source.gateways() {
            self.add_gateway_table(gateways);
        }
    }

    fn add_gateway_table(&mut self, gateways: &super::gateway::GatewayTable) {
        self.uses_gateways = true;
        self.gateway_programs
            .extend(gateways.programs().into_iter().map(str::to_owned));
        self.languages.extend(
            gateways
                .units()
                .into_iter()
                .filter_map(|unit| YonLanguage::from_path(Path::new(unit.relative_path()))),
        );
    }

    fn process_requirements(&self, runtimes: &HandlerRuntimePrograms) -> Vec<RuntimeRequirement> {
        let mut requirements = Vec::new();
        self.push_interpreter_requirements(runtimes, &mut requirements);
        self.push_system_runtime_requirements(&mut requirements);
        self.push_ahead_of_time_compilers(&mut requirements);
        self.push_gateway_programs(&mut requirements);
        requirements.sort_by_key(RuntimeRequirement::key);
        requirements
    }

    fn push_gateway_programs(&self, requirements: &mut Vec<RuntimeRequirement>) {
        for program in &self.gateway_programs {
            requirements.push(RuntimeRequirement::gateway_program(program));
        }
    }

    fn firecracker_requirements(
        &self,
        policy: &FirecrackerIsolation,
        isolation: &YonIsolationPolicy,
    ) -> Result<Vec<RuntimeRequirement>, Failure> {
        self.check_no_gateways()?;
        self.languages
            .iter()
            .try_for_each(|language| isolation.validate_backend_yon_language(*language))?;
        Ok(self.firecracker_driver_requirements(policy))
    }

    fn check_no_gateways(&self) -> Result<(), Failure> {
        if self.uses_gateways {
            return Err(Failure::one(diagnostic(
                2010,
                "Gateways require the Yon process backend.",
                Some(String::from(
                    "The Firecracker transport does not yet transfer invocation-scoped resource capabilities.",
                )),
                None,
            )));
        }
        Ok(())
    }

    fn firecracker_driver_requirements(
        &self,
        policy: &FirecrackerIsolation,
    ) -> Vec<RuntimeRequirement> {
        if self.languages.is_empty() {
            return Vec::new();
        }
        vec![RuntimeRequirement::firecracker_driver(policy)]
    }

    fn uses(&self, language: YonLanguage) -> bool {
        self.languages.contains(&language)
    }

    fn uses_any(&self, languages: &[YonLanguage]) -> bool {
        languages
            .iter()
            .any(|language| self.languages.contains(language))
    }

    fn push_if(
        requirements: &mut Vec<RuntimeRequirement>,
        needed: bool,
        requirement: RuntimeRequirement,
    ) {
        if needed {
            requirements.push(requirement);
        }
    }

    fn push_interpreter_requirements(
        &self,
        runtimes: &HandlerRuntimePrograms,
        requirements: &mut Vec<RuntimeRequirement>,
    ) {
        Self::push_if(
            requirements,
            self.uses_any(&[YonLanguage::JavaScript, YonLanguage::TypeScript]),
            RuntimeRequirement::javascript(runtimes, self.uses(YonLanguage::TypeScript)),
        );
        Self::push_if(
            requirements,
            self.uses(YonLanguage::Python),
            RuntimeRequirement::configured(
                RuntimeRequirementKey::Python,
                &runtimes.python,
                HandlerRuntimePrograms::DEFAULT_PYTHON,
                "Yon Python runtime",
                "YON_PYTHON_RUNTIME",
                "Install Python or correct YON_PYTHON_RUNTIME.",
                ProbeKind::Python,
            ),
        );
        Self::push_if(
            requirements,
            self.uses(YonLanguage::Ruby),
            RuntimeRequirement::configured(
                RuntimeRequirementKey::Ruby,
                &runtimes.ruby,
                HandlerRuntimePrograms::DEFAULT_RUBY,
                "Yon Ruby runtime",
                "YON_RUBY_RUNTIME",
                "Install Ruby 3.3 or newer, or correct YON_RUBY_RUNTIME.",
                ProbeKind::RubyVersion,
            ),
        );
    }

    fn push_system_runtime_requirements(&self, requirements: &mut Vec<RuntimeRequirement>) {
        let has_java = self.uses(YonLanguage::Java);
        let has_kotlin = self.uses(YonLanguage::Kotlin);
        Self::push_if(
            requirements,
            self.uses_any(&[YonLanguage::Java, YonLanguage::Groovy, YonLanguage::Kotlin]),
            RuntimeRequirement::fixed(
                RuntimeRequirementKey::Java,
                "java",
                "Yon Java runtime 'java'",
                "Install a JDK that provides the Java source launcher and runtime.",
                ProbeKind::Java {
                    source_launcher: has_java,
                },
            ),
        );
        Self::push_if(
            requirements,
            self.uses(YonLanguage::Php),
            RuntimeRequirement::fixed(
                RuntimeRequirementKey::Php,
                "php-cgi",
                RuntimeRequirement::PHP_LABEL,
                RuntimeRequirement::PHP_HELP,
                ProbeKind::Version(&["-v"]),
            ),
        );
        Self::push_if(
            requirements,
            self.uses(YonLanguage::Groovy),
            RuntimeRequirement::groovy_runtime(),
        );
        Self::push_if(
            requirements,
            self.uses(YonLanguage::Lua),
            RuntimeRequirement::fixed(
                RuntimeRequirementKey::Lua,
                crate::lua_source::LuaSource::RUNTIME,
                RuntimeRequirement::LUA_LABEL,
                RuntimeRequirement::LUA_HELP,
                ProbeKind::LuaVersion,
            ),
        );
        Self::push_if(
            requirements,
            has_kotlin,
            RuntimeRequirement::fixed(
                RuntimeRequirementKey::KotlinCompiler,
                "kotlinc",
                RuntimeRequirement::KOTLIN_LABEL,
                RuntimeRequirement::KOTLIN_HELP,
                ProbeKind::Version(&["-version"]),
            ),
        );
        Self::push_if(
            requirements,
            self.uses_any(&[YonLanguage::CSharp, YonLanguage::FSharp]),
            RuntimeRequirement::fixed(
                RuntimeRequirementKey::Dotnet,
                "dotnet",
                RuntimeRequirement::DOTNET_LABEL,
                RuntimeRequirement::DOTNET_HELP,
                ProbeKind::DotnetBuild,
            ),
        );
    }

    fn push_ahead_of_time_compilers(&self, requirements: &mut Vec<RuntimeRequirement>) {
        Self::push_if(
            requirements,
            self.uses(YonLanguage::Rust),
            RuntimeRequirement::fixed(
                RuntimeRequirementKey::RustCompiler,
                "cargo",
                RuntimeRequirement::CARGO_LABEL,
                RuntimeRequirement::RUST_HELP,
                ProbeKind::Version(&["--version"]),
            ),
        );
        Self::push_if(
            requirements,
            self.uses(YonLanguage::Dart),
            RuntimeRequirement::fixed(
                RuntimeRequirementKey::DartCompiler,
                "dart",
                RuntimeRequirement::DART_LABEL,
                RuntimeRequirement::DART_HELP,
                ProbeKind::Version(&["--version"]),
            ),
        );
        Self::push_if(
            requirements,
            self.uses(YonLanguage::Go),
            RuntimeRequirement::fixed(
                RuntimeRequirementKey::GoCompiler,
                "go",
                RuntimeRequirement::GO_LABEL,
                RuntimeRequirement::GO_HELP,
                ProbeKind::GoVersion,
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    #[cfg(unix)]
    use super::{ProbeKind, RuntimeProbe, RuntimeProbeResult, RuntimeProbeState};
    use super::{RuntimeRequirement, RuntimeRequirementKey, RuntimeRequirements};
    #[cfg(unix)]
    use crate::handler::EnvironmentPolicy;
    use crate::handler::{HandlerRuntimePrograms, YonIsolationPolicy};
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    fn gateway_requirements(programs: &[&str]) -> RuntimeRequirements {
        RuntimeRequirements {
            languages: BTreeSet::new(),
            gateway_programs: programs
                .iter()
                .map(|program| (*program).to_owned())
                .collect(),
            uses_gateways: true,
        }
    }

    #[test]
    fn gateway_programs_are_required_on_the_path_without_running_them() {
        let requirements = gateway_requirements(&["sh", "tachyon-missing-gateway-program"])
            .resolve(
                &HandlerRuntimePrograms::default(),
                &YonIsolationPolicy::Process,
            )
            .expect("requirements");
        let environment = crate::handler::EnvironmentPolicy::default();
        let results = super::RuntimeProbe::all_sync(requirements, &environment).expect("probes");
        let failure = super::RuntimeProbeResult::failure(&results).expect("missing program");
        let rendered = failure.to_string();
        assert!(rendered.contains("TY2112"), "{rendered}");
        assert!(
            rendered.contains("tachyon-missing-gateway-program"),
            "{rendered}"
        );
        assert!(!rendered.contains("'sh'"), "{rendered}");
    }

    #[test]
    fn windows_preflight_accepts_only_programs_that_spawn_can_run() {
        let directory = tempfile::tempdir().expect("path directory");
        for name in ["ruby.cmd", "python3.exe", "node.exe", "tool.exe", "perl"] {
            std::fs::write(directory.path().join(name), "").expect("candidate");
        }
        for (program, runnable) in [
            ("ruby", false),
            ("python3.11", false),
            ("perl", false),
            ("node", true),
            ("tool.exe", true),
        ] {
            let found = super::RuntimeProbe::windows_executable(
                directory.path(),
                std::path::Path::new(program),
            );
            assert_eq!(found, runnable, "{program}");
        }
    }

    #[test]
    fn gateways_are_refused_under_firecracker_isolation() {
        let firecracker = YonIsolationPolicy::from_lookup(|name| match name {
            "YON_ISOLATION" => Some("firecracker".into()),
            "YON_FIRECRACKER_DRIVER" => Some("/usr/bin/true".into()),
            _ => None,
        })
        .expect("firecracker policy");
        let result =
            gateway_requirements(&[]).resolve(&HandlerRuntimePrograms::default(), &firecracker);
        let Err(failure) = result else {
            panic!("gateways need the process backend");
        };
        assert!(failure.to_string().contains("TY2010"), "{failure}");
    }

    #[test]
    fn language_requirements_are_complete_deduplicated_and_safe() {
        let requirements = RuntimeRequirements::from_extensions([
            "js", "ts", "py", "java", "groovy", "php", "kt", "cs", "fs", "rs", "go", "rb", "lua",
        ]);
        let runtimes = HandlerRuntimePrograms {
            javascript: PathBuf::from("/secret/canary-bun"),
            python: PathBuf::from("/secret/canary-python"),
            ruby: PathBuf::from("/secret/canary-ruby"),
        };
        let resolved = requirements
            .resolve(&runtimes, &YonIsolationPolicy::Process)
            .expect("requirements");
        let keys = resolved
            .iter()
            .map(RuntimeRequirement::key)
            .collect::<Vec<_>>();
        assert_eq!(
            keys,
            vec![
                RuntimeRequirementKey::JavaScript,
                RuntimeRequirementKey::Python,
                RuntimeRequirementKey::Ruby,
                RuntimeRequirementKey::Lua,
                RuntimeRequirementKey::Java,
                RuntimeRequirementKey::GroovyRuntime,
                RuntimeRequirementKey::Php,
                RuntimeRequirementKey::KotlinCompiler,
                RuntimeRequirementKey::Dotnet,
                RuntimeRequirementKey::RustCompiler,
                RuntimeRequirementKey::GoCompiler,
            ]
        );
        for value in &resolved {
            assert!(!value.label().contains("secret"), "{}", value.label());
            assert!(!value.help().contains("secret"), "{}", value.help());
        }
        assert_eq!(count_key(&keys, RuntimeRequirementKey::Java), 1);
        assert_eq!(count_key(&keys, RuntimeRequirementKey::JavaScript), 1);
    }

    fn count_key(keys: &[RuntimeRequirementKey], wanted: RuntimeRequirementKey) -> usize {
        keys.iter().filter(|key| **key == wanted).count()
    }

    #[test]
    fn go_must_be_version_1_27_or_newer() {
        for (output, supported) in [
            ("go version go1.27.1 darwin/arm64", true),
            ("go version go1.27 linux/amd64", true),
            ("go version go1.30rc1 linux/amd64", true),
            ("go version go1.26.4 linux/amd64", false),
            ("go version go1.9 linux/amd64", false),
            ("not a go version", false),
        ] {
            assert_eq!(
                super::RuntimeProbe::supports_go_version(output),
                supported,
                "{output}"
            );
        }
    }

    #[test]
    fn ruby_must_be_version_3_3_or_newer() {
        for (output, supported) in [
            ("3.3.0", true),
            ("3.4.7", true),
            ("4.0.6", true),
            ("3.2.9", false),
            ("2.7.8", false),
            ("not a ruby version", false),
            ("", false),
        ] {
            assert_eq!(
                super::RuntimeProbe::supports_ruby_version(output),
                supported,
                "{output}"
            );
        }
    }

    #[test]
    fn lua_must_be_version_5_4_or_newer() {
        for (output, supported) in [
            ("5.4", true),
            ("5.5", true),
            ("5.3", false),
            ("5.1", false),
            ("", false),
        ] {
            assert_eq!(
                super::RuntimeProbe::supports_lua_version(output),
                supported,
                "{output}"
            );
        }
    }

    #[test]
    fn only_node_24_5_or_newer_and_bun_honour_the_outgoing_proxy_switch() {
        for (output, honoured) in [
            ("v24.5.0\n", true),
            ("v24.21.0", true),
            ("v25.0.0", true),
            ("v26.6.5", true),
            ("bun\n", true),
            ("v24.4.1", false),
            ("v24.0.0", false),
            ("v23.5.0", false),
            ("v22.21.0", false),
            ("v18.15.0", false),
            ("24.21.0", false),
            ("1.3.14", false),
            ("deno 2.1.0", false),
            ("", false),
        ] {
            assert_eq!(
                super::RuntimeProbe::honours_node_proxy_switch(output),
                honoured,
                "{output}"
            );
        }
    }

    #[test]
    fn the_outgoing_proxy_switch_check_reuses_the_javascript_runtime_label() {
        let runtimes = HandlerRuntimePrograms {
            javascript: PathBuf::from("node"),
            ..HandlerRuntimePrograms::default()
        };
        let requirement = RuntimeRequirement::node_proxy_switch(&runtimes);
        assert_eq!(requirement.key(), RuntimeRequirementKey::NodeProxySwitch);
        assert_eq!(requirement.label(), "Yon JavaScript runtime 'node'");
        assert!(requirement.help().contains("NODE_USE_ENV_PROXY=1"));
        assert!(requirement.help().contains("Install Node.js 24.5 or newer"));
    }

    #[test]
    fn unknown_extensions_require_nothing() {
        assert_eq!(
            RuntimeRequirements::from_extensions(["pl", "html"])
                .languages
                .len(),
            0
        );
    }

    fn go_requirement(program: PathBuf) -> RuntimeRequirement {
        let requirements = RuntimeRequirements::from_extensions(["go"])
            .resolve(
                &HandlerRuntimePrograms::default(),
                &YonIsolationPolicy::Process,
            )
            .expect("requirements");
        let mut requirement = requirements.into_iter().next().expect("go requirement");
        requirement.program = program;
        requirement
    }

    #[test]
    fn a_missing_go_fails_preflight_with_the_missing_toolchain_error() {
        let requirement = go_requirement(PathBuf::from("tachyon-missing-go-compiler"));
        let environment = crate::handler::EnvironmentPolicy::default();
        let results =
            super::RuntimeProbe::all_sync(vec![requirement], &environment).expect("probe");
        let rendered = super::RuntimeProbeResult::failure(&results)
            .expect("missing go")
            .to_string();
        assert!(rendered.contains("TY2112"), "{rendered}");
        assert!(
            rendered.contains("Required Yon Go compiler 'go' was not found."),
            "{rendered}"
        );
        assert!(rendered.contains("Install Go 1.27 or newer"), "{rendered}");
    }

    #[cfg(unix)]
    #[test]
    fn an_old_go_is_not_usable() {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempfile::tempdir().expect("fake go directory");
        let old_go = directory.path().join("go");
        std::fs::write(
            &old_go,
            "#!/bin/sh\necho 'go version go1.26.4 linux/amd64'\n",
        )
        .expect("fake go");
        std::fs::set_permissions(&old_go, std::fs::Permissions::from_mode(0o700))
            .expect("permissions");
        let environment = EnvironmentPolicy::default();
        let results =
            RuntimeProbe::all_sync(vec![go_requirement(old_go)], &environment).expect("probe");
        assert_eq!(results[0].state(), RuntimeProbeState::Incomplete);
        let rendered = RuntimeProbeResult::failure(&results)
            .expect("old go")
            .to_string();
        assert!(rendered.contains("TY2101"), "{rendered}");
    }

    #[cfg(unix)]
    fn fake_program(directory: &std::path::Path, name: &str, script: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;

        let program = directory.join(name);
        std::fs::write(&program, script).expect("fake program");
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700))
            .expect("permissions");
        program
    }

    #[cfg(unix)]
    fn slow_toolchain_requirements(directory: &std::path::Path) -> Vec<RuntimeRequirement> {
        let slow_compiler = fake_program(
            directory,
            "slow-compiler",
            "#!/bin/sh\n/bin/sleep 6\necho 'compiler 2.1.10'\n",
        );
        let slow_dotnet = fake_program(
            directory,
            "slow-dotnet",
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 10.0.302; exit 0; fi\n/bin/sleep 5\nexit 0\n",
        );
        vec![
            RuntimeRequirement {
                key: RuntimeRequirementKey::KotlinCompiler,
                program: slow_compiler,
                command_label: String::from("kotlinc"),
                label: String::from(RuntimeRequirement::KOTLIN_LABEL),
                help: String::from(RuntimeRequirement::KOTLIN_HELP),
                probe: ProbeKind::Version(&["-version"]),
            },
            RuntimeRequirement {
                key: RuntimeRequirementKey::Dotnet,
                program: slow_dotnet,
                command_label: String::from("dotnet"),
                label: String::from(RuntimeRequirement::DOTNET_LABEL),
                help: String::from(RuntimeRequirement::DOTNET_HELP),
                probe: ProbeKind::DotnetBuild,
            },
        ]
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn slow_working_toolchains_each_get_their_own_time_limit() {
        let directory = tempfile::tempdir().expect("fake toolchain directory");
        let requirements = slow_toolchain_requirements(directory.path());
        let blocking_requirements = requirements.clone();
        let synchronous = tokio::task::spawn_blocking(move || {
            RuntimeProbe::all_sync(blocking_requirements, &EnvironmentPolicy::default())
        });
        let asynchronous = RuntimeProbe::all(requirements, &EnvironmentPolicy::default())
            .await
            .expect("asynchronous probe");
        let synchronous = synchronous
            .await
            .expect("probe thread")
            .expect("synchronous probe");
        for results in [&asynchronous, &synchronous] {
            let states: Vec<RuntimeProbeState> =
                results.iter().map(RuntimeProbeResult::state).collect();
            assert_eq!(
                states,
                vec![RuntimeProbeState::Ready, RuntimeProbeState::Ready]
            );
            assert!(RuntimeProbeResult::failure(results).is_none());
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn runtime_only_dotnet_fake_fails_the_real_build_probe_without_path_disclosure() {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;

        let directory = tempfile::tempdir().expect("fake runtime directory");
        let canary = directory.path().join("secret-runtime-only-dotnet-canary");
        fs::write(
            &canary,
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 10.0.302; exit 0; fi\nif [ \"$1\" = \"--list-runtimes\" ]; then echo 'Microsoft.NETCore.App 10.0.0 [/fake]'; exit 0; fi\nexit 9\n",
        )
        .expect("fake runtime");
        fs::set_permissions(&canary, fs::Permissions::from_mode(0o700)).expect("permissions");
        let requirement = RuntimeRequirement {
            key: RuntimeRequirementKey::Dotnet,
            program: canary.clone(),
            command_label: String::from("dotnet"),
            label: String::from("Yon C# runtime 'dotnet'"),
            help: String::from("Install the .NET SDK and Microsoft.NETCore.App runtime."),
            probe: ProbeKind::DotnetBuild,
        };
        let environment = EnvironmentPolicy::default();

        let asynchronous = RuntimeProbe::all(vec![requirement.clone()], &environment)
            .await
            .expect("asynchronous probe");
        let synchronous =
            RuntimeProbe::all_sync(vec![requirement], &environment).expect("synchronous probe");
        for results in [&asynchronous, &synchronous] {
            assert_eq!(results[0].state(), RuntimeProbeState::Incomplete);
            let failure = RuntimeProbeResult::failure(results).expect("readiness failure");
            let rendered = failure.to_string();
            assert!(rendered.contains("TY2101"), "{rendered}");
            assert!(
                !rendered.contains(canary.to_string_lossy().as_ref()),
                "{rendered}"
            );
        }
    }
}
