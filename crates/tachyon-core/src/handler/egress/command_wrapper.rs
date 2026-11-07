use std::ffi::OsString;
use std::path::PathBuf;
use tokio::process::Command;

#[derive(Clone, Debug)]
pub(super) struct CommandWrapper {
    program: PathBuf,
    prefix: Vec<OsString>,
}

impl CommandWrapper {
    #[cfg_attr(not(unix), expect(dead_code))]
    pub(super) const fn new(program: PathBuf, prefix: Vec<OsString>) -> Self {
        Self { program, prefix }
    }

    pub(super) fn wrap(&self, command: &mut Command) {
        let original = command.as_std();
        let mut wrapped = Command::new(&self.program);
        wrapped
            .args(&self.prefix)
            .arg(original.get_program())
            .args(original.get_args())
            .env_clear()
            .envs(
                original
                    .get_envs()
                    .filter_map(|(name, value)| Some((name, value?))),
            );
        Self::keep_directory(original, &mut wrapped);
        crate::handler::HandlerSupervisor::pipe_all_streams(&mut wrapped);
        *command = wrapped;
    }

    fn keep_directory(original: &std::process::Command, wrapped: &mut Command) {
        if let Some(directory) = original.get_current_dir() {
            wrapped.current_dir(directory);
        }
    }
}
