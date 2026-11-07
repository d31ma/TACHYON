use super::host::{native_tool_failure, run_tool};
use super::native_running_app::NativeRunningApp;
use super::{DevelopmentOrigin, NativeBuildResult};
use crate::Failure;
use command_group::AsyncCommandGroup;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tachyon_contracts::NativeTarget;

#[derive(Debug)]
pub struct NativeAppLaunch {
    running: NativeRunningApp,
    port: String,
}

impl NativeAppLaunch {
    const SIMULATOR_VARIABLE: &'static str = "TAC_IOS_SIMULATOR";
    const BOOTED_SIMULATOR: &'static str = "booted";
    const SDK_VARIABLES: [&'static str; 2] = ["ANDROID_HOME", "ANDROID_SDK_ROOT"];
    const ANDROID_ACTIVITY: &'static str = ".MainActivity";
    const STOP_WAIT: Duration = Duration::from_secs(5);

    pub async fn start(
        build: &NativeBuildResult,
        development: &DevelopmentOrigin,
    ) -> Result<Self, Failure> {
        let port = development.port().to_string();
        let running = Self::running(build, development.target(), &port).await?;
        Ok(Self { running, port })
    }

    pub fn has_exited(&mut self) -> bool {
        Self::exited(&mut self.running)
    }

    pub async fn stop(self) {
        Self::stop_running(self.running, &self.port).await;
    }

    fn exited(running: &mut NativeRunningApp) -> bool {
        match running {
            NativeRunningApp::Desktop(child) => child.try_wait().ok().flatten().is_some(),
            NativeRunningApp::Simulator { .. } | NativeRunningApp::Emulator { .. } => false,
        }
    }

    async fn stop_running(running: NativeRunningApp, port: &str) {
        match running {
            NativeRunningApp::Desktop(mut child) => {
                let _signalled = child.start_kill();
                let _waited = tokio::time::timeout(Self::STOP_WAIT, child.wait()).await;
            }
            NativeRunningApp::Simulator { device, identifier } => {
                let _stopped =
                    run_tool("xcrun", &["simctl", "terminate", &device, &identifier]).await;
            }
            NativeRunningApp::Emulator { adb, package } => {
                let adb = adb.to_string_lossy();
                let _stopped = run_tool(&adb, &["shell", "am", "force-stop", &package]).await;
                let forwarded = format!("tcp:{port}");
                let _removed = run_tool(&adb, &["reverse", "--remove", &forwarded]).await;
            }
        }
    }

    async fn running(
        build: &NativeBuildResult,
        target: NativeTarget,
        port: &str,
    ) -> Result<NativeRunningApp, Failure> {
        let bundle = build.application_bundle();
        let name = build.executable_name();
        match target {
            NativeTarget::Macos => Self::desktop(&bundle.join("Contents/MacOS").join(name)),
            NativeTarget::Windows => Self::desktop(&bundle.join("bin").join(format!("{name}.exe"))),
            NativeTarget::Linux => Self::desktop(&bundle.join("bin").join(name)),
            NativeTarget::Ios => Self::simulator(build).await,
            NativeTarget::Android => Self::emulator(build, port).await,
        }
    }

    fn desktop(executable: &Path) -> Result<NativeRunningApp, Failure> {
        tokio::process::Command::new(executable)
            .kill_on_drop(true)
            .group_spawn()
            .map(|child| NativeRunningApp::Desktop(Box::new(child)))
            .map_err(|error| {
                native_tool_failure(
                    1605,
                    &format!("Cannot start the app {}: {error}", executable.display()),
                )
            })
    }

    async fn simulator(build: &NativeBuildResult) -> Result<NativeRunningApp, Failure> {
        let device = std::env::var(Self::SIMULATOR_VARIABLE)
            .ok()
            .filter(|device| !device.trim().is_empty())
            .unwrap_or_else(|| String::from(Self::BOOTED_SIMULATOR));
        let bundle = build.application_bundle().to_string_lossy().into_owned();
        let identifier = String::from(build.launch_identifier());
        run_tool("xcrun", &["simctl", "install", &device, &bundle]).await?;
        run_tool(
            "xcrun",
            &[
                "simctl",
                "launch",
                "--terminate-running-process",
                &device,
                &identifier,
            ],
        )
        .await?;
        Ok(NativeRunningApp::Simulator { device, identifier })
    }

    async fn emulator(build: &NativeBuildResult, port: &str) -> Result<NativeRunningApp, Failure> {
        let adb = Self::adb();
        let tool = adb.to_string_lossy().into_owned();
        let forwarded = format!("tcp:{port}");
        let package = String::from(build.launch_identifier());
        let activity = format!("{package}/{}", Self::ANDROID_ACTIVITY);
        let apk = build.application_bundle().to_string_lossy().into_owned();
        run_tool(&tool, &["reverse", &forwarded, &forwarded]).await?;
        run_tool(&tool, &["install", "-r", &apk]).await?;
        run_tool(&tool, &["shell", "am", "start", "-W", "-n", &activity]).await?;
        Ok(NativeRunningApp::Emulator { adb, package })
    }

    fn adb() -> PathBuf {
        Self::SDK_VARIABLES
            .iter()
            .filter_map(std::env::var_os)
            .map(|sdk| PathBuf::from(sdk).join("platform-tools").join("adb"))
            .find(|adb| adb.is_file())
            .unwrap_or_else(|| PathBuf::from("adb"))
    }
}
