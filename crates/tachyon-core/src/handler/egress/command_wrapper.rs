use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use tokio::process::Command;

#[derive(Clone, Debug)]
pub(super) struct CommandWrapper {
    program: PathBuf,
    prefix: Vec<OsString>,
}

impl CommandWrapper {
    const PATH: &'static str = "PATH";

    #[cfg_attr(not(unix), expect(dead_code))]
    pub(super) const fn new(program: PathBuf, prefix: Vec<OsString>) -> Self {
        Self { program, prefix }
    }

    pub(super) fn wrap(&self, command: &mut Command) {
        let original = command.as_std();
        let Some(resolved) = Self::resolve(original) else {
            command.env(Self::PATH, "");
            return;
        };
        let mut wrapped = Command::new(&self.program);
        wrapped
            .args(&self.prefix)
            .arg(resolved)
            .args(original.get_args())
            .env_clear();
        for (name, value) in original.get_envs() {
            if let Some(value) = value {
                wrapped.env(name, value);
            }
        }
        if let Some(directory) = original.get_current_dir() {
            wrapped.current_dir(directory);
        }
        crate::handler::HandlerSupervisor::pipe_all_streams(&mut wrapped);
        *command = wrapped;
    }

    fn resolve(command: &std::process::Command) -> Option<PathBuf> {
        let program = Path::new(command.get_program());
        if program.components().count() > 1 {
            let candidate = command.get_current_dir().map_or_else(
                || program.to_path_buf(),
                |directory| directory.join(program),
            );
            return Self::existing(candidate);
        }
        let search = Self::search_path(command)?;
        std::env::split_paths(&search)
            .map(|directory| directory.join(program))
            .find_map(Self::existing)
    }

    fn existing(candidate: PathBuf) -> Option<PathBuf> {
        let absolute = std::path::absolute(candidate).ok()?;
        absolute.is_file().then_some(absolute)
    }

    fn search_path(command: &std::process::Command) -> Option<OsString> {
        let listed = command
            .get_envs()
            .find(|(name, _)| *name == OsStr::new(Self::PATH))
            .map(|(_, value)| value.map(OsStr::to_os_string));
        listed.unwrap_or_else(|| std::env::var_os(Self::PATH))
    }
}
