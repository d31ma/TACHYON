use crate::external_command::{PartialOutput, ToolError};
use std::time::Duration;

pub(crate) struct NodeFailure {
    message: String,
    stdout_before_deadline: Option<Vec<u8>>,
}

impl NodeFailure {
    const LONGEST_ARGUMENT: usize = 60;
    const SHOWN_OUTPUT_BYTES: usize = 4 * 1_024;

    pub(crate) fn stdout_before_deadline(&self) -> Option<&[u8]> {
        self.stdout_before_deadline.as_deref()
    }

    pub(super) fn new(
        description: &str,
        deadline: Duration,
        error: ToolError,
        retried: bool,
    ) -> Self {
        let mut failure = Self::explained(description, deadline, error);
        if retried {
            failure.message.push_str(
                "\nIts first run had also printed its report and then not exited, so this was its one retry",
            );
        }
        failure
    }

    fn explained(description: &str, deadline: Duration, error: ToolError) -> Self {
        match error {
            ToolError::TimedOut(partial) => Self::timed_out(description, deadline, partial),
            other => Self {
                message: format!("{description}: {other}"),
                stdout_before_deadline: None,
            },
        }
    }

    fn timed_out(description: &str, deadline: Duration, partial: PartialOutput) -> Self {
        let message = format!(
            "{description} did not finish within {} seconds, so its process tree was killed\nstdout before the deadline:\n{}\nstderr before the deadline:\n{}",
            deadline.as_secs_f64(),
            Self::last_text(&partial.stdout),
            Self::last_text(&partial.stderr),
        );
        Self {
            message,
            stdout_before_deadline: Some(partial.stdout),
        }
    }

    pub(super) fn describe(command: &std::process::Command) -> String {
        let program = command.get_program().to_string_lossy().into_owned();
        let words: Vec<String> = std::iter::once(program)
            .chain(command.get_args().map(Self::shortened))
            .collect();
        let line = words.join(" ");
        command.get_current_dir().map_or_else(
            || format!("`{line}`"),
            |folder| format!("`{line}` in {}", folder.display()),
        )
    }

    fn shortened(argument: &std::ffi::OsStr) -> String {
        let argument = argument.to_string_lossy();
        if argument.chars().count() <= Self::LONGEST_ARGUMENT {
            return argument.into_owned();
        }
        let start: String = argument.chars().take(Self::LONGEST_ARGUMENT).collect();
        format!("{start}...")
    }

    fn last_text(bytes: &[u8]) -> String {
        let start = bytes.len().saturating_sub(Self::SHOWN_OUTPUT_BYTES);
        String::from_utf8_lossy(&bytes[start..]).into_owned()
    }
}

impl std::fmt::Display for NodeFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::fmt::Debug for NodeFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for NodeFailure {}
