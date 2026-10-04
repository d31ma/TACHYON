use super::{
    DotnetVersionOutcome, ProbeKind, RuntimeProbeResult, RuntimeProbeState, RuntimeRequirement,
};
use crate::Failure;
use crate::external_command::{ToolError, ToolOutput, run, run_sync};
use crate::handler::EnvironmentPolicy;
use std::fs;
use std::path::Path;
use std::time::Duration;
use tokio::process::Command;

pub(crate) struct RuntimeProbe;

impl RuntimeProbe {
    pub(in crate::handler) async fn all(
        requirements: Vec<RuntimeRequirement>,
        environment: &EnvironmentPolicy,
    ) -> Result<Vec<RuntimeProbeResult>, Failure> {
        Self::probe_each(requirements, environment).await
    }

    pub(crate) fn all_sync(
        requirements: Vec<RuntimeRequirement>,
        environment: &EnvironmentPolicy,
    ) -> Result<Vec<RuntimeProbeResult>, Failure> {
        Self::probe_each_sync(requirements, environment)
    }

    pub(super) fn supports_ruby_version(output: &str) -> bool {
        Self::version_at_least(output, RuntimeRequirement::OLDEST_RUBY_VERSION)
    }

    pub(super) fn supports_lua_version(output: &str) -> bool {
        Self::version_at_least(output, RuntimeRequirement::OLDEST_LUA_VERSION)
    }

    fn version_at_least(output: &str, oldest: (u32, u32)) -> bool {
        let mut parts = output.trim().split('.');
        let major = parts.next().and_then(|part| part.parse::<u32>().ok());
        let minor = parts.next().and_then(|part| part.parse::<u32>().ok());
        major.zip(minor).is_some_and(|version| version >= oldest)
    }

    pub(crate) fn supports_go_version(output: &str) -> bool {
        output
            .split_whitespace()
            .find_map(|word| word.strip_prefix("go1."))
            .is_some_and(Self::supported_go_minor)
    }

    pub(super) fn windows_executable(directory: &Path, program: &Path) -> bool {
        directory.join(Self::windows_name(program)).is_file()
    }

    async fn probe_each(
        requirements: Vec<RuntimeRequirement>,
        environment: &EnvironmentPolicy,
    ) -> Result<Vec<RuntimeProbeResult>, Failure> {
        let mut results = Vec::with_capacity(requirements.len());
        for requirement in requirements {
            let state = Self::probe_one(&requirement, environment).await?;
            results.push(RuntimeProbeResult::new(requirement, state));
        }
        Ok(results)
    }

    fn probe_each_sync(
        requirements: Vec<RuntimeRequirement>,
        environment: &EnvironmentPolicy,
    ) -> Result<Vec<RuntimeProbeResult>, Failure> {
        let mut results = Vec::with_capacity(requirements.len());
        for requirement in requirements {
            let state = Self::probe_one_sync(&requirement, environment)?;
            results.push(RuntimeProbeResult::new(requirement, state));
        }
        Ok(results)
    }

    async fn probe_one(
        requirement: &RuntimeRequirement,
        environment: &EnvironmentPolicy,
    ) -> Result<RuntimeProbeState, Failure> {
        let known = Self::probe_without_running(requirement.probe, &requirement.program);
        Self::run_unless_known(known, requirement, environment).await
    }

    fn probe_one_sync(
        requirement: &RuntimeRequirement,
        environment: &EnvironmentPolicy,
    ) -> Result<RuntimeProbeState, Failure> {
        let known = Self::probe_without_running(requirement.probe, &requirement.program);
        Self::run_unless_known_sync(known, requirement, environment)
    }

    async fn run_unless_known(
        known: Option<RuntimeProbeState>,
        requirement: &RuntimeRequirement,
        environment: &EnvironmentPolicy,
    ) -> Result<RuntimeProbeState, Failure> {
        match known {
            Some(state) => Ok(state),
            None => Self::run_probe(requirement, environment).await,
        }
    }

    fn run_unless_known_sync(
        known: Option<RuntimeProbeState>,
        requirement: &RuntimeRequirement,
        environment: &EnvironmentPolicy,
    ) -> Result<RuntimeProbeState, Failure> {
        match known {
            Some(state) => Ok(state),
            None => Self::run_probe_sync(requirement, environment),
        }
    }

    async fn run_probe(
        requirement: &RuntimeRequirement,
        environment: &EnvironmentPolicy,
    ) -> Result<RuntimeProbeState, Failure> {
        let mut prepared = requirement.prepare()?;
        environment.apply_std(&mut prepared.command);
        let mut command = Command::from(prepared.command);
        let result = Self::run_limited(&mut command, RuntimeRequirement::PROBE_TIME_LIMIT).await;
        Self::state_after_run(requirement, result, environment).await
    }

    fn run_probe_sync(
        requirement: &RuntimeRequirement,
        environment: &EnvironmentPolicy,
    ) -> Result<RuntimeProbeState, Failure> {
        let mut prepared = requirement.prepare()?;
        environment.apply_std(&mut prepared.command);
        let result =
            Self::run_limited_sync(&mut prepared.command, RuntimeRequirement::PROBE_TIME_LIMIT);
        Self::state_after_run_sync(requirement, result, environment)
    }

    async fn run_limited(command: &mut Command, limit: Duration) -> Result<ToolOutput, ToolError> {
        run(command, limit, RuntimeRequirement::PROBE_OUTPUT_BYTES).await
    }

    fn run_limited_sync(
        command: &mut std::process::Command,
        limit: Duration,
    ) -> Result<ToolOutput, ToolError> {
        run_sync(command, limit, RuntimeRequirement::PROBE_OUTPUT_BYTES)
    }

    async fn state_after_run(
        requirement: &RuntimeRequirement,
        result: Result<ToolOutput, ToolError>,
        environment: &EnvironmentPolicy,
    ) -> Result<RuntimeProbeState, Failure> {
        match requirement.probe {
            ProbeKind::DotnetBuild => {
                Self::probe_dotnet_build(requirement, result, environment).await
            }
            ProbeKind::GoVersion => Ok(Self::go_version_state(result)),
            ProbeKind::RubyVersion => Ok(Self::ruby_version_state(result)),
            ProbeKind::LuaVersion => Ok(Self::lua_version_state(result)),
            _ => Ok(Self::probe_state(result)),
        }
    }

    fn state_after_run_sync(
        requirement: &RuntimeRequirement,
        result: Result<ToolOutput, ToolError>,
        environment: &EnvironmentPolicy,
    ) -> Result<RuntimeProbeState, Failure> {
        match requirement.probe {
            ProbeKind::DotnetBuild => {
                Self::probe_dotnet_build_sync(requirement, result, environment)
            }
            ProbeKind::GoVersion => Ok(Self::go_version_state(result)),
            ProbeKind::RubyVersion => Ok(Self::ruby_version_state(result)),
            ProbeKind::LuaVersion => Ok(Self::lua_version_state(result)),
            _ => Ok(Self::probe_state(result)),
        }
    }

    async fn probe_dotnet_build(
        requirement: &RuntimeRequirement,
        initial: Result<ToolOutput, ToolError>,
        environment: &EnvironmentPolicy,
    ) -> Result<RuntimeProbeState, Failure> {
        let outcome = Self::dotnet_version_outcome(initial);
        Self::build_when_sdk_found(outcome, requirement, environment).await
    }

    fn probe_dotnet_build_sync(
        requirement: &RuntimeRequirement,
        initial: Result<ToolOutput, ToolError>,
        environment: &EnvironmentPolicy,
    ) -> Result<RuntimeProbeState, Failure> {
        let outcome = Self::dotnet_version_outcome(initial);
        Self::build_when_sdk_found_sync(outcome, requirement, environment)
    }

    async fn build_when_sdk_found(
        outcome: DotnetVersionOutcome,
        requirement: &RuntimeRequirement,
        environment: &EnvironmentPolicy,
    ) -> Result<RuntimeProbeState, Failure> {
        match outcome {
            DotnetVersionOutcome::Finished(state) => Ok(state),
            DotnetVersionOutcome::SdkMajor(sdk_major) => {
                Self::run_dotnet_build(requirement, sdk_major, environment).await
            }
        }
    }

    fn build_when_sdk_found_sync(
        outcome: DotnetVersionOutcome,
        requirement: &RuntimeRequirement,
        environment: &EnvironmentPolicy,
    ) -> Result<RuntimeProbeState, Failure> {
        match outcome {
            DotnetVersionOutcome::Finished(state) => Ok(state),
            DotnetVersionOutcome::SdkMajor(sdk_major) => {
                Self::run_dotnet_build_sync(requirement, sdk_major, environment)
            }
        }
    }

    async fn run_dotnet_build(
        requirement: &RuntimeRequirement,
        sdk_major: u32,
        environment: &EnvironmentPolicy,
    ) -> Result<RuntimeProbeState, Failure> {
        let mut prepared = requirement.prepare_dotnet_build(sdk_major)?;
        prepared.apply_dotnet_build_environment(environment);
        let mut command = Command::from(prepared.command);
        let limit = RuntimeRequirement::DOTNET_BUILD_TIME_LIMIT;
        let result = Self::run_limited(&mut command, limit).await;
        Ok(Self::probe_state(result))
    }

    fn run_dotnet_build_sync(
        requirement: &RuntimeRequirement,
        sdk_major: u32,
        environment: &EnvironmentPolicy,
    ) -> Result<RuntimeProbeState, Failure> {
        let mut prepared = requirement.prepare_dotnet_build(sdk_major)?;
        prepared.apply_dotnet_build_environment(environment);
        let limit = RuntimeRequirement::DOTNET_BUILD_TIME_LIMIT;
        let result = Self::run_limited_sync(&mut prepared.command, limit);
        Ok(Self::probe_state(result))
    }

    fn dotnet_version_outcome(initial: Result<ToolOutput, ToolError>) -> DotnetVersionOutcome {
        let sdk_major = initial.as_ref().ok().and_then(Self::dotnet_sdk_major);
        let initial_state = Self::probe_state(initial);
        Self::outcome_for(initial_state, sdk_major)
    }

    fn outcome_for(
        initial_state: RuntimeProbeState,
        sdk_major: Option<u32>,
    ) -> DotnetVersionOutcome {
        match (initial_state, sdk_major) {
            (RuntimeProbeState::Ready, Some(sdk_major)) => {
                DotnetVersionOutcome::SdkMajor(sdk_major)
            }
            (RuntimeProbeState::Ready, None) => {
                DotnetVersionOutcome::Finished(RuntimeProbeState::Incomplete)
            }
            (state, _) => DotnetVersionOutcome::Finished(state),
        }
    }

    fn dotnet_sdk_major(output: &ToolOutput) -> Option<u32> {
        if !output.status.success() {
            return None;
        }
        Self::parsed_major(&output.stdout)
    }

    fn parsed_major(stdout: &[u8]) -> Option<u32> {
        String::from_utf8_lossy(stdout)
            .trim()
            .split('.')
            .next()?
            .parse::<u32>()
            .ok()
            .filter(|major| *major != 0)
    }

    fn probe_state(result: Result<ToolOutput, ToolError>) -> RuntimeProbeState {
        result.map_or_else(Self::error_state, |output| Self::output_state(&output))
    }

    fn output_state(output: &ToolOutput) -> RuntimeProbeState {
        if output.status.success() {
            return RuntimeProbeState::Ready;
        }
        RuntimeProbeState::Incomplete
    }

    fn error_state(error: ToolError) -> RuntimeProbeState {
        match error {
            ToolError::Spawn(error) if error.kind() == std::io::ErrorKind::NotFound => {
                RuntimeProbeState::Missing
            }
            _ => RuntimeProbeState::Incomplete,
        }
    }

    fn go_version_state(result: Result<ToolOutput, ToolError>) -> RuntimeProbeState {
        let supported = result.as_ref().is_ok_and(|output| {
            Self::supports_go_version(&String::from_utf8_lossy(&output.stdout))
        });
        let state = Self::probe_state(result);
        Self::unless_unsupported(state, supported)
    }

    fn ruby_version_state(result: Result<ToolOutput, ToolError>) -> RuntimeProbeState {
        let supported = result.as_ref().is_ok_and(|output| {
            Self::supports_ruby_version(&String::from_utf8_lossy(&output.stdout))
        });
        let state = Self::probe_state(result);
        Self::unless_unsupported(state, supported)
    }

    fn lua_version_state(result: Result<ToolOutput, ToolError>) -> RuntimeProbeState {
        let supported = result.as_ref().is_ok_and(|output| {
            Self::supports_lua_version(&String::from_utf8_lossy(&output.stdout))
        });
        let state = Self::probe_state(result);
        Self::unless_unsupported(state, supported)
    }

    fn unless_unsupported(state: RuntimeProbeState, supported: bool) -> RuntimeProbeState {
        if state == RuntimeProbeState::Ready && !supported {
            return RuntimeProbeState::Incomplete;
        }
        state
    }

    fn supported_go_minor(version: &str) -> bool {
        let minor: String = version.chars().take_while(char::is_ascii_digit).collect();
        minor
            .parse::<u32>()
            .is_ok_and(|minor| minor >= RuntimeRequirement::OLDEST_GO_MINOR_VERSION)
    }

    fn probe_without_running(probe: ProbeKind, program: &Path) -> Option<RuntimeProbeState> {
        match probe {
            ProbeKind::File => Some(Self::probe_file(program)),
            ProbeKind::OnPath => Some(Self::probe_on_path(program)),
            _ => None,
        }
    }

    fn probe_file(program: &Path) -> RuntimeProbeState {
        fs::symlink_metadata(program).map_or_else(
            |error| Self::missing_file_state(&error),
            |metadata| Self::regular_file_state(&metadata),
        )
    }

    fn regular_file_state(metadata: &fs::Metadata) -> RuntimeProbeState {
        if metadata.is_file() && !metadata.file_type().is_symlink() {
            return RuntimeProbeState::Ready;
        }
        RuntimeProbeState::Incomplete
    }

    fn missing_file_state(error: &std::io::Error) -> RuntimeProbeState {
        if error.kind() == std::io::ErrorKind::NotFound {
            return RuntimeProbeState::Missing;
        }
        RuntimeProbeState::Incomplete
    }

    fn probe_on_path(program: &Path) -> RuntimeProbeState {
        let paths = std::env::var_os("PATH").unwrap_or_default();
        Self::found_state(Self::on_path(&paths, program))
    }

    pub(crate) fn on_path(paths: &std::ffi::OsStr, program: &Path) -> bool {
        std::env::split_paths(paths).any(|directory| Self::runnable_in(&directory, program))
    }

    fn found_state(found: bool) -> RuntimeProbeState {
        if found {
            return RuntimeProbeState::Ready;
        }
        RuntimeProbeState::Missing
    }

    fn runnable_in(directory: &Path, program: &Path) -> bool {
        if cfg!(windows) {
            return Self::windows_executable(directory, program);
        }
        Self::executable_file(&directory.join(program))
    }

    fn windows_name(program: &Path) -> std::ffi::OsString {
        let mut name = program.as_os_str().to_os_string();
        if !name.as_encoded_bytes().contains(&b'.') {
            name.push(".exe");
        }
        name
    }

    #[cfg(unix)]
    fn executable_file(path: &Path) -> bool {
        fs::metadata(path).is_ok_and(|metadata| Self::is_executable(&metadata))
    }

    #[cfg(unix)]
    fn is_executable(metadata: &fs::Metadata) -> bool {
        use std::os::unix::fs::PermissionsExt;
        metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
    }

    #[cfg(not(unix))]
    fn executable_file(path: &Path) -> bool {
        path.is_file()
    }
}
