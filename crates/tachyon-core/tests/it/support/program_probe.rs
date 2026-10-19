use std::process::{Command, Stdio};

pub struct ProgramProbe;

impl ProgramProbe {
    const WINDOWS_SHELL: &str = "cmd";
    const WINDOWS_SHELL_RUN: [&str; 2] = ["/D", "/C"];

    /// Kotlin and Groovy ship only `.bat` launchers on Windows, which a spawn by name cannot
    /// find, so there the probe asks the shell, which searches `PATHEXT`.
    pub fn answers(program: &str, flag: &str) -> bool {
        Self::command(program)
            .arg(flag)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }

    fn command(program: &str) -> Command {
        if cfg!(windows) {
            let mut shell = Command::new(Self::WINDOWS_SHELL);
            shell.args(Self::WINDOWS_SHELL_RUN).arg(program);
            return shell;
        }
        Command::new(program)
    }
}
