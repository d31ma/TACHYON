use crate::external_command::ToolError;
use std::process::{Command, Stdio};

/// Node 24 on Windows ARM64 sometimes deadlocks at process exit: the main
/// thread waits forever to join a V8 platform worker that missed its wake-up
/// (seen in a debugger on 2026-10-17). A driver caught there has already
/// printed its whole JSON report, so it is run once more instead of failing.
pub(super) struct ExitHang;

impl ExitHang {
    pub(super) fn seen(error: &ToolError) -> bool {
        let ToolError::TimedOut(partial) = error else {
            return false;
        };
        let stdout = String::from_utf8_lossy(&partial.stdout);
        stdout.ends_with('\n')
            && stdout.lines().last().is_some_and(|line| {
                serde_json::from_str::<serde_json::Value>(line)
                    .is_ok_and(|report| report.is_object() || report.is_array())
            })
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
}

#[cfg(test)]
mod tests {
    use super::ExitHang;
    use crate::external_command::{PartialOutput, ToolError};

    fn timed_out(stdout: &str) -> ToolError {
        ToolError::TimedOut(PartialOutput {
            stdout: stdout.as_bytes().to_vec(),
            stderr: Vec::new(),
        })
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
