use super::probe_memory::ProbeMemory;
use std::process::{Command, Stdio};

pub struct ProgramProbe;

impl ProgramProbe {
    const WINDOWS_SHELL: &str = "cmd";
    const WINDOWS_SHELL_RUN: [&str; 2] = ["/D", "/C"];

    /// Kotlin and Groovy ship only `.bat` launchers on Windows, which a spawn by name cannot
    /// find, so there a program the spawn misses is asked for through the shell, which searches
    /// `PATHEXT`.
    pub fn answers(program: &str, flag: &str) -> bool {
        let memory = ProbeMemory::for_this_run();
        Self::run_unless_remembered(&memory, program, flag)
    }

    fn run_unless_remembered(memory: &ProbeMemory, program: &str, flag: &str) -> bool {
        if memory.answered(program, flag) {
            return true;
        }
        memory.remember(program, flag, Self::runs(program, flag))
    }

    fn runs(program: &str, flag: &str) -> bool {
        let shell = Self::shell_command(program);
        [Some(Command::new(program)), shell]
            .into_iter()
            .flatten()
            .any(|mut command| Self::succeeds(command.arg(flag)))
    }

    fn succeeds(command: &mut Command) -> bool {
        command
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }

    fn shell_command(program: &str) -> Option<Command> {
        cfg!(windows).then(|| {
            let mut shell = Command::new(Self::WINDOWS_SHELL);
            shell.args(Self::WINDOWS_SHELL_RUN).arg(program);
            shell
        })
    }
}
