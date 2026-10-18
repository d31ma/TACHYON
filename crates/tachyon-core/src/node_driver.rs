mod exit_hang;
mod node_failure;

use crate::external_command::{ToolError, ToolOutput};
use exit_hang::ExitHang;
use node_failure::NodeFailure;
use std::process::{Output, Stdio};
use std::time::Duration;

pub(crate) struct NodeDriver;

impl NodeDriver {
    pub(crate) const DEADLINE: Duration = Duration::from_mins(2);
    const PROBE_DEADLINE: Duration = Duration::from_secs(30);
    const OUTPUT_BYTES: usize = 64 * 1_024 * 1_024;

    pub(crate) fn command() -> std::process::Command {
        let mut command = std::process::Command::new("node");
        command.stdin(Stdio::null());
        command
    }

    pub(crate) fn async_command() -> tokio::process::Command {
        let mut command = tokio::process::Command::new("node");
        command.stdin(Stdio::null());
        command
    }

    pub(crate) fn available() -> bool {
        Self::output_within(Self::command().arg("--version"), Self::PROBE_DEADLINE)
            .is_ok_and(|output| output.status.success())
    }

    pub(crate) fn output(command: &mut std::process::Command) -> Result<Output, NodeFailure> {
        Self::output_within(command, Self::DEADLINE)
    }

    pub(crate) fn output_within(
        command: &mut std::process::Command,
        deadline: Duration,
    ) -> Result<Output, NodeFailure> {
        command.stdin(Stdio::null());
        let description = NodeFailure::describe(command);
        let mut retry = ExitHang::copy(command);
        let first = crate::external_command::run_sync(command, deadline, Self::OUTPUT_BYTES);
        let result = match first {
            Err(error) if ExitHang::seen(&error) => {
                ExitHang::announce(&description);
                crate::external_command::run_sync(&mut retry, deadline, Self::OUTPUT_BYTES)
                    .map_err(|error| (error, true))
            }
            other => other.map_err(|error| (error, false)),
        };
        Self::settled(result, &description, deadline)
    }

    pub(crate) async fn run(command: &mut tokio::process::Command) -> Result<Output, NodeFailure> {
        Self::run_within(command, Self::DEADLINE).await
    }

    pub(crate) async fn run_within(
        command: &mut tokio::process::Command,
        deadline: Duration,
    ) -> Result<Output, NodeFailure> {
        command.stdin(Stdio::null());
        let description = NodeFailure::describe(command.as_std());
        let mut retry = tokio::process::Command::from(ExitHang::copy(command.as_std()));
        let first = crate::external_command::run(command, deadline, Self::OUTPUT_BYTES).await;
        let result = match first {
            Err(error) if ExitHang::seen(&error) => {
                ExitHang::announce(&description);
                crate::external_command::run(&mut retry, deadline, Self::OUTPUT_BYTES)
                    .await
                    .map_err(|error| (error, true))
            }
            other => other.map_err(|error| (error, false)),
        };
        Self::settled(result, &description, deadline)
    }

    fn settled(
        result: Result<ToolOutput, (ToolError, bool)>,
        description: &str,
        deadline: Duration,
    ) -> Result<Output, NodeFailure> {
        result
            .map(Self::finished)
            .map_err(|(error, retried)| NodeFailure::new(description, deadline, error, retried))
    }

    fn finished(output: ToolOutput) -> Output {
        Output {
            status: output.status,
            stdout: output.stdout,
            stderr: output.stderr,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::NodeDriver;
    use std::time::Duration;

    #[test]
    fn a_driver_that_never_exits_fails_with_its_output_so_far() {
        if !NodeDriver::available() {
            return;
        }
        let started = std::time::Instant::now();
        let failure = NodeDriver::output_within(
            NodeDriver::command().args([
                "--eval",
                "console.log('report printed'); setInterval(() => {}, 1000)",
            ]),
            Duration::from_secs(2),
        )
        .expect_err("a driver that never exits must fail");
        assert!(started.elapsed() < Duration::from_secs(10));
        assert_eq!(
            failure.stdout_before_deadline(),
            Some("report printed\n".as_bytes())
        );
        let message = failure.to_string();
        assert!(
            message.contains("did not finish within 2 seconds"),
            "{message}"
        );
        assert!(message.contains("report printed"), "{message}");
        assert!(!message.contains("one retry"), "{message}");
    }

    struct Drivers;

    impl Drivers {
        const HANGS_ONCE_AFTER_ITS_REPORT: &str = "const fs = require('node:fs')\n\
             console.log(JSON.stringify({ report: 'printed' }))\n\
             if (fs.existsSync(process.env.TAC_FIRST_RUN)) process.exit(0)\n\
             fs.writeFileSync(process.env.TAC_FIRST_RUN, '')\n\
             setInterval(() => {}, 1000)";
    }

    #[test]
    fn a_driver_that_hangs_after_its_report_runs_once_more() {
        if !NodeDriver::available() {
            return;
        }
        let folder = tempfile::tempdir().expect("folder");
        let first_run = folder.path().join("first-run");
        let output = NodeDriver::output_within(
            NodeDriver::command()
                .args(["--eval", Drivers::HANGS_ONCE_AFTER_ITS_REPORT])
                .env("TAC_FIRST_RUN", &first_run),
            Duration::from_secs(2),
        )
        .expect("the second run exits");
        assert!(first_run.exists());
        assert!(output.status.success());
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "{\"report\":\"printed\"}\n"
        );
    }

    #[tokio::test]
    async fn an_async_driver_that_hangs_after_its_report_runs_once_more() {
        if !NodeDriver::available() {
            return;
        }
        let folder = tempfile::tempdir().expect("folder");
        let first_run = folder.path().join("first-run");
        let mut command = NodeDriver::async_command();
        command
            .args(["--eval", Drivers::HANGS_ONCE_AFTER_ITS_REPORT])
            .env("TAC_FIRST_RUN", &first_run);
        let output = NodeDriver::run_within(&mut command, Duration::from_secs(2))
            .await
            .expect("the second run exits");
        assert!(first_run.exists());
        assert!(output.status.success());
    }

    #[test]
    fn a_driver_that_always_hangs_after_its_report_gets_only_one_retry() {
        if !NodeDriver::available() {
            return;
        }
        let started = std::time::Instant::now();
        let failure = NodeDriver::output_within(
            NodeDriver::command()
                .args(["--eval", "console.log('[1]'); setInterval(() => {}, 1000)"]),
            Duration::from_secs(2),
        )
        .expect_err("a driver that never exits must fail");
        assert!(started.elapsed() < Duration::from_secs(20));
        let message = failure.to_string();
        assert!(message.contains("this was its one retry"), "{message}");
        assert_eq!(failure.stdout_before_deadline(), Some("[1]\n".as_bytes()));
    }

    #[test]
    fn a_driver_cannot_wait_for_input() {
        if !NodeDriver::available() {
            return;
        }
        let output = NodeDriver::output(NodeDriver::command().args([
            "--eval",
            "let read = 0; process.stdin.on('data', (part) => { read += part.length }); process.stdin.on('end', () => console.log(read))",
        ]))
        .expect("stdin is closed, so the driver ends");
        assert!(output.status.success());
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "0");
    }

    #[tokio::test]
    async fn an_async_driver_reports_a_missing_program_clearly() {
        let mut command = NodeDriver::async_command();
        command
            .arg("--version")
            .current_dir("/tachyon-folder-that-does-not-exist");
        let failure = NodeDriver::run(&mut command)
            .await
            .expect_err("a missing folder cannot start");
        let message = failure.to_string();
        assert!(message.starts_with("`node --version` in "), "{message}");
        assert!(message.contains("cannot spawn tool"), "{message}");
        assert!(failure.stdout_before_deadline().is_none());
    }
}
