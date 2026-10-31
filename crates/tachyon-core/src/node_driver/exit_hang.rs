use crate::external_command::{ToolError, ToolOutput};
use std::process::{Command, Stdio};

/// Node 24 on Windows sometimes fails at process exit after a driver has printed
/// its whole JSON report: on ARM64 the main thread can wait forever to join a V8
/// platform worker that missed its wake-up (seen in a debugger on 2026-10-17),
/// and a platform worker can wake an event-loop handle that exit is already
/// closing, which aborts in libuv (seen on windows-latest on 2026-10-19). A
/// driver caught there is run once more instead of failing. Node 24.20.0 fixed
/// the abort (nodejs/node#61999), and CI uses a Node release with that fix.
pub(super) struct ExitHang;

impl ExitHang {
    const CLOSING_HANDLE_ABORT: &'static str = "!(handle->flags & UV_HANDLE_CLOSING)";

    pub(super) fn seen(error: &ToolError) -> bool {
        let ToolError::TimedOut(partial) = error else {
            return false;
        };
        Self::printed_report(&partial.stdout)
    }

    pub(super) fn crashed(output: &ToolOutput) -> bool {
        let stderr = String::from_utf8_lossy(&output.stderr);
        [
            output.status.success(),
            stderr.contains(Self::CLOSING_HANDLE_ABORT),
            Self::printed_report(&output.stdout),
        ] == [false, true, true]
    }

    fn printed_report(stdout: &[u8]) -> bool {
        let stdout = String::from_utf8_lossy(stdout);
        let last_report = stdout.lines().last().is_some_and(|line| {
            serde_json::from_str::<serde_json::Value>(line)
                .is_ok_and(|report| report.is_object() || report.is_array())
        });
        [stdout.ends_with('\n'), last_report] == [true, true]
    }

    pub(super) fn copy(command: &Command) -> Command {
        let mut copy = Command::new(command.get_program());
        copy.args(command.get_args()).stdin(Stdio::null());
        if let Some(folder) = command.get_current_dir() {
            copy.current_dir(folder);
        }
        for (name, value) in command.get_envs() {
            match value {
                Some(value) => copy.env(name, value),
                None => copy.env_remove(name),
            };
        }
        copy
    }

    pub(super) fn announce(description: &str) {
        eprintln!(
            "{description} printed its report but did not exit before its deadline; running it once more"
        );
    }

    pub(super) fn announce_crash(description: &str) {
        eprintln!(
            "{description} printed its report but Node aborted while it exited; running it once more"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::ExitHang;
    use crate::external_command::{PartialOutput, ToolError, ToolOutput};
    use std::process::ExitStatus;

    fn timed_out(stdout: &str) -> ToolError {
        ToolError::TimedOut(PartialOutput {
            stdout: stdout.as_bytes().to_vec(),
            stderr: Vec::new(),
        })
    }

    struct Exits;

    impl Exits {
        const ABORT: &'static str = "Assertion failed: !(handle->flags & UV_HANDLE_CLOSING), file src\\win\\async.c, line 94\n";

        #[cfg(unix)]
        fn status(code: u8) -> ExitStatus {
            std::os::unix::process::ExitStatusExt::from_raw(i32::from(code) << 8)
        }

        #[cfg(windows)]
        fn status(code: u8) -> ExitStatus {
            std::os::windows::process::ExitStatusExt::from_raw(u32::from(code))
        }

        fn output(code: u8, stdout: &str, stderr: &str) -> ToolOutput {
            ToolOutput {
                status: Self::status(code),
                stdout: stdout.as_bytes().to_vec(),
                stderr: stderr.as_bytes().to_vec(),
            }
        }
    }

    #[test]
    fn only_a_libuv_abort_after_a_finished_report_is_an_exit_crash() {
        let report = "{\"ok\":true}\n";
        assert!(ExitHang::crashed(&Exits::output(3, report, Exits::ABORT)));
        assert!(!ExitHang::crashed(&Exits::output(0, report, Exits::ABORT)));
        assert!(!ExitHang::crashed(&Exits::output(
            3,
            report,
            "Error: lost\n"
        )));
        assert!(!ExitHang::crashed(&Exits::output(
            3,
            "phase one\n",
            Exits::ABORT
        )));
    }

    #[test]
    fn only_a_finished_json_report_is_an_exit_hang() {
        assert!(ExitHang::seen(&timed_out("{\"ok\":true}\n")));
        assert!(ExitHang::seen(&timed_out("phase one\n[1,2]\n")));
        assert!(!ExitHang::seen(&timed_out("")));
        assert!(!ExitHang::seen(&timed_out("report printed\n")));
        assert!(!ExitHang::seen(&timed_out("{\"ok\":true}")));
        assert!(!ExitHang::seen(&timed_out("{\"ok\":\n")));
        assert!(!ExitHang::seen(&timed_out("42\n")));
        assert!(!ExitHang::seen(&ToolError::Wait(std::io::Error::other(
            "lost"
        ))));
    }

    #[test]
    fn a_copy_keeps_the_arguments_folder_and_environment() {
        let mut command = std::process::Command::new("node");
        command
            .args(["drive.mjs", "--fast"])
            .current_dir("/drivers")
            .env("TAC_SET", "1")
            .env_remove("TAC_UNSET");
        let copy = ExitHang::copy(&command);
        assert_eq!(copy.get_program(), "node");
        assert_eq!(copy.get_args().collect::<Vec<_>>(), ["drive.mjs", "--fast"]);
        assert_eq!(
            copy.get_current_dir(),
            Some(std::path::Path::new("/drivers"))
        );
        let envs: Vec<_> = copy.get_envs().collect();
        assert!(envs.contains(&("TAC_SET".as_ref(), Some("1".as_ref()))));
        assert!(envs.contains(&("TAC_UNSET".as_ref(), None)));
    }
}
