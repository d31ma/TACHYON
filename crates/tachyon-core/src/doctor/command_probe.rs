use super::ToolchainState;
use crate::external_command::{ToolOutput, run_sync};
use std::process::Command;
use std::time::Duration;

pub(crate) struct CommandProbe;

impl CommandProbe {
    const DEADLINE: Duration = Duration::from_secs(10);
    const OUTPUT_BYTES: usize = 64 * 1_024;

    pub(crate) fn version_state(program: &str, arguments: &[&str]) -> ToolchainState {
        Self::run(program, arguments).map_or(ToolchainState::Missing, ToolchainState::Ready)
    }

    pub(crate) fn run(program: &str, arguments: &[&str]) -> Option<String> {
        let text = Self::output(program, arguments)?;
        let first_line = text.lines().next()?;
        Some(first_line.trim().to_owned())
    }

    pub(crate) fn output(program: &str, arguments: &[&str]) -> Option<String> {
        let mut command = Command::new(program);
        command.args(arguments);
        let result = run_sync(&mut command, Self::DEADLINE, Self::OUTPUT_BYTES).ok()?;
        let result = Self::successful(result)?;
        Some(Self::readable_output(&result))
    }

    fn successful(result: ToolOutput) -> Option<ToolOutput> {
        result.status.success().then_some(result)
    }

    fn readable_output(result: &ToolOutput) -> String {
        let text = String::from_utf8_lossy(&result.stdout);
        if text.trim().is_empty() {
            return String::from_utf8_lossy(&result.stderr).into_owned();
        }
        text.into_owned()
    }
}
