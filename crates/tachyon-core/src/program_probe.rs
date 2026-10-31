use crate::probe_memory::ProbeMemory;
use std::process::{Command, Stdio};

pub(crate) struct ProgramProbe;

impl ProgramProbe {
    const WINDOWS_SHELL: &'static str = "cmd";
    const WINDOWS_SHELL_RUN: [&'static str; 2] = ["/D", "/C"];

    /// Kotlin ships only `.bat` launchers on Windows and an emsdk may too, and a spawn by name
    /// cannot find those, so a program is asked for through the launcher Tachyon builds with and,
    /// on Windows, through the shell, which searches `PATHEXT`.
    pub(crate) fn answers(program: &str, flag: &str) -> bool {
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
        let launcher = Some(crate::kotlin_home::KotlinHome::command(program));
        [launcher, Self::shell_command(program)]
            .into_iter()
            .flatten()
            .any(|mut command| Self::succeeds(command.arg(flag)))
    }

    fn succeeds(command: &mut Command) -> bool {
        command
            .stdin(Stdio::null())
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
