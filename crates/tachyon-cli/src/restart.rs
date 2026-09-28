use std::io::Write as _;
use std::process::Command;
use tachyon_core::{Failure, ToolRestart};

pub(crate) struct SelfRestart;

impl SelfRestart {
    const FAILURE_CODE: u16 = 1321;

    pub(crate) fn replace_process(tool_restart: &ToolRestart) -> Failure {
        println!("{}", ToolRestart::RESTART_MESSAGE);
        let _flush_result = std::io::stdout().flush();
        let mut command = Command::new(tool_restart.executable());
        command.args(std::env::args_os().skip(1));
        Self::run_new_process(command)
    }

    #[cfg(unix)]
    fn run_new_process(mut command: Command) -> Failure {
        use std::os::unix::process::CommandExt as _;
        Self::failure(&command.exec())
    }

    #[cfg(not(unix))]
    fn run_new_process(mut command: Command) -> Failure {
        match command.status() {
            Ok(status) => std::process::exit(status.code().unwrap_or(1)),
            Err(error) => Self::failure(&error),
        }
    }

    fn failure(error: &std::io::Error) -> Failure {
        crate::cli_failure(
            Self::FAILURE_CODE,
            format!("Tachyon was updated on disk, but the new version cannot start: {error}."),
            "Rebuild ty and start the preview again.",
        )
    }
}
