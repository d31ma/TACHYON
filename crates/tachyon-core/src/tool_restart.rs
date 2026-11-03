use crate::Failure;
use crate::compiler::ToolBuild;
use crate::failure::diagnostic;
use futures_util::future::OptionFuture;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug)]
pub struct ToolRestart {
    state: Arc<ToolRestartState>,
}

#[derive(Debug)]
struct ToolRestartState {
    executable: PathBuf,
    check_arguments: Vec<OsString>,
    handled_build: Mutex<String>,
    refused: AtomicBool,
    requested: CancellationToken,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum ToolUpdate {
    Unchanged,
    Restart,
    Refused(String),
    StillRefused,
}

impl ToolRestart {
    const SETTLE: Duration = Duration::from_millis(500);
    const PROBE_LIMIT: Duration = Duration::from_mins(3);
    const NOTICE_CODE: u16 = 1320;
    pub const RESTART_MESSAGE: &str = "Tachyon was updated on disk. Restarting the preview…";

    #[must_use]
    pub fn watch(executable: PathBuf, check_arguments: Vec<OsString>) -> Self {
        let started_build = ToolBuild::id(Some(&executable));
        Self {
            state: Arc::new(ToolRestartState {
                executable,
                check_arguments,
                handled_build: Mutex::new(started_build),
                refused: AtomicBool::new(false),
                requested: CancellationToken::new(),
            }),
        }
    }

    #[must_use]
    pub fn executable(&self) -> &Path {
        &self.state.executable
    }

    #[must_use]
    pub fn is_requested(&self) -> bool {
        self.state.requested.is_cancelled()
    }

    pub async fn requested(&self) {
        self.state.requested.cancelled().await;
    }

    pub(crate) fn request(&self) {
        self.state.requested.cancel();
    }

    pub(crate) async fn check(&self, sources_changed: bool) -> ToolUpdate {
        let settled = self.settled_change().await;
        let should_probe = self.should_probe(settled.is_some(), sources_changed);
        let probed = OptionFuture::from(should_probe.then(|| self.probe())).await;
        let update = ToolUpdate::after_probe(probed, settled.is_some());
        self.accept(settled);
        self.remember_refusal(&update);
        update
    }

    async fn settled_change(&self) -> Option<String> {
        let noticed_build = self.changed_build()?;
        tokio::time::sleep(Self::SETTLE).await;
        self.changed_build()
            .filter(|current_build| *current_build == noticed_build)
    }

    fn should_probe(&self, new_build: bool, sources_changed: bool) -> bool {
        let refused = self.state.refused.load(Ordering::Acquire);
        let waiting_for_a_fix = [sources_changed, refused] == [true, true];
        [new_build, waiting_for_a_fix].contains(&true)
    }

    fn accept(&self, settled: Option<String>) {
        if let Some(build) = settled {
            *self.handled_build() = build;
        }
    }

    fn remember_refusal(&self, update: &ToolUpdate) {
        let waiting = update.waits_for_a_fix(self.state.refused.load(Ordering::Acquire));
        self.state.refused.store(waiting, Ordering::Release);
    }

    fn handled_build(&self) -> std::sync::MutexGuard<'_, String> {
        self.state
            .handled_build
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    fn changed_build(&self) -> Option<String> {
        let current_build = ToolBuild::id(Some(&self.state.executable));
        (current_build != *self.handled_build()).then_some(current_build)
    }

    async fn probe(&self) -> Result<(), String> {
        let mut command = tokio::process::Command::new(&self.state.executable);
        command
            .args(&self.state.check_arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let output = tokio::time::timeout(Self::PROBE_LIMIT, command.output())
            .await
            .map_err(|_| {
                format!(
                    "its start-up check did not finish within {} seconds",
                    Self::PROBE_LIMIT.as_secs()
                )
            })?
            .map_err(|error| format!("it cannot run ({error})"))?;
        if output.status.success() {
            return Ok(());
        }
        let error_output = String::from_utf8_lossy(&output.stderr);
        Err(format!(
            "its start-up check failed with {}\n{}",
            output.status,
            error_output.trim()
        )
        .trim_end()
        .to_owned())
    }
}

impl ToolUpdate {
    fn after_probe(probed: Option<Result<(), String>>, new_build: bool) -> Self {
        match (probed, new_build) {
            (None, _) => Self::Unchanged,
            (Some(Ok(())), _) => Self::Restart,
            (Some(Err(reason)), true) => Self::Refused(reason),
            (Some(Err(_)), false) => Self::StillRefused,
        }
    }

    fn waits_for_a_fix(&self, waiting: bool) -> bool {
        match self {
            Self::Unchanged => waiting,
            Self::Restart => false,
            Self::Refused(_) | Self::StillRefused => true,
        }
    }

    pub(crate) fn notice(&self) -> Option<Failure> {
        self.notice_text().map(|(message, help)| {
            Failure::one(diagnostic(
                ToolRestart::NOTICE_CODE,
                message,
                Some(String::from(help)),
                None,
            ))
        })
    }

    fn notice_text(&self) -> Option<(String, &'static str)> {
        match self {
            Self::Unchanged | Self::StillRefused => None,
            Self::Restart => Some((
                String::from(ToolRestart::RESTART_MESSAGE),
                "Open pages reload when the new version is ready.",
            )),
            Self::Refused(reason) => Some((
                format!(
                    "Tachyon was updated on disk, but the new version does not start: {reason}."
                ),
                "The preview keeps running the previous version. Fix the project or rebuild ty, and the preview tries again.",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::{ToolRestart, ToolUpdate};
    use std::ffi::OsString;
    use std::fs;

    #[tokio::test]
    async fn an_unchanged_executable_needs_no_restart() {
        let directory = tempfile::tempdir().expect("directory");
        let executable = directory.path().join("ty");
        fs::write(&executable, "first").expect("executable");
        let restart = ToolRestart::watch(executable, vec![OsString::from("--version")]);
        assert_eq!(restart.check(false).await, ToolUpdate::Unchanged);
        assert!(!restart.is_requested());
    }

    #[tokio::test]
    async fn a_new_executable_that_does_not_start_is_refused_once() {
        let directory = tempfile::tempdir().expect("directory");
        let executable = directory.path().join("ty");
        fs::write(&executable, "first").expect("executable");
        let restart = ToolRestart::watch(executable.clone(), vec![OsString::from("--version")]);
        fs::write(&executable, "second version").expect("new executable");
        let refused = restart.check(false).await;
        assert!(
            matches!(&refused, ToolUpdate::Refused(reason) if reason.contains("cannot run")),
            "{refused:?}"
        );
        let notice = refused.notice().expect("notice").to_string();
        assert!(notice.contains("TY1320"), "{notice}");
        assert!(notice.contains("does not start"), "{notice}");
        assert_eq!(restart.check(false).await, ToolUpdate::Unchanged);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_new_executable_that_starts_is_restarted_once() {
        use std::os::unix::fs::PermissionsExt as _;
        let directory = tempfile::tempdir().expect("directory");
        let executable = directory.path().join("ty");
        fs::write(&executable, "#!/bin/sh\nexit 0\n").expect("executable");
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).expect("mode");
        let restart = ToolRestart::watch(executable.clone(), vec![OsString::from("--version")]);
        fs::write(&executable, "#!/bin/sh\necho updated\nexit 0\n").expect("new executable");
        let update = restart.check(false).await;
        assert_eq!(update, ToolUpdate::Restart);
        let notice = update.notice().expect("notice").to_string();
        assert!(notice.contains(ToolRestart::RESTART_MESSAGE), "{notice}");
        assert_eq!(restart.check(false).await, ToolUpdate::Unchanged);
        assert!(!restart.is_requested());
        restart.request();
        restart.requested().await;
        assert!(restart.is_requested());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_refused_executable_restarts_after_the_project_is_fixed() {
        use std::os::unix::fs::PermissionsExt as _;
        let directory = tempfile::tempdir().expect("directory");
        let executable = directory.path().join("ty");
        let broken = directory.path().join("broken");
        fs::write(&executable, "#!/bin/sh\nexit 0\n").expect("executable");
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).expect("mode");
        let check = vec![OsString::from("--check-project"), broken.clone().into()];
        let restart = ToolRestart::watch(executable.clone(), check);
        fs::write(&broken, "").expect("broken project");
        fs::write(
            &executable,
            "#!/bin/sh\n[ -f \"$2\" ] && echo 'error[TY1333]: unused' >&2 && exit 1\nexit 0\n",
        )
        .expect("new executable");
        let refused = restart.check(false).await;
        assert!(
            matches!(&refused, ToolUpdate::Refused(reason) if reason.contains("TY1333")),
            "{refused:?}"
        );
        assert_eq!(restart.check(true).await, ToolUpdate::StillRefused);
        fs::remove_file(&broken).expect("fixed project");
        assert_eq!(restart.check(false).await, ToolUpdate::Unchanged);
        assert_eq!(restart.check(true).await, ToolUpdate::Restart);
        assert_eq!(restart.check(true).await, ToolUpdate::Unchanged);
    }
}
