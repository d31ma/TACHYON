use super::host::{native_tool_failure, run_tool};
use super::{DevelopmentOrigin, NativeBuildResult};
use crate::Failure;
use command_group::{AsyncCommandGroup, AsyncGroupChild};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tachyon_contracts::NativeTarget;

#[derive(Debug)]
enum RunningApp {
    Desktop(Box<AsyncGroupChild>),
    Simulator { device: String, identifier: String },
    Emulator { adb: PathBuf, package: String },
}

#[derive(Debug)]
pub struct NativeAppLaunch {
    running: RunningApp,
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
        let running = match development.target() {
            NativeTarget::Linux | NativeTarget::Macos | NativeTarget::Windows => {
                Self::desktop(&Self::executable(build, development.target()))?
            }
            NativeTarget::Ios => Self::simulator(build).await?,
            NativeTarget::Android => Self::emulator(build, &port).await?,
        };
        Ok(Self { running, port })
    }

    pub fn has_exited(&mut self) -> bool {
        match &mut self.running {
            RunningApp::Desktop(child) => child.try_wait().ok().flatten().is_some(),
            RunningApp::Simulator { .. } | RunningApp::Emulator { .. } => false,
        }
    }

    pub async fn stop(self) {
        match self.running {
            RunningApp::Desktop(mut child) => {
                let _signalled = child.start_kill();
                let _waited = tokio::time::timeout(Self::STOP_WAIT, child.wait()).await;
            }
            RunningApp::Simulator { device, identifier } => {
                let _stopped =
                    run_tool("xcrun", &["simctl", "terminate", &device, &identifier]).await;
            }
            RunningApp::Emulator { adb, package } => {
                let adb = adb.to_string_lossy();
                let _stopped = run_tool(&adb, &["shell", "am", "force-stop", &package]).await;
                let forwarded = format!("tcp:{}", self.port);
                let _removed = run_tool(&adb, &["reverse", "--remove", &forwarded]).await;
            }
        }
    }

    fn executable(build: &NativeBuildResult, target: NativeTarget) -> PathBuf {
        let bundle = build.application_bundle();
        let name = build.executable_name();
        match target {
            NativeTarget::Macos => bundle.join("Contents/MacOS").join(name),
            NativeTarget::Windows => bundle.join("bin").join(format!("{name}.exe")),
            NativeTarget::Linux | NativeTarget::Ios | NativeTarget::Android => {
                bundle.join("bin").join(name)
            }
        }
    }

    fn desktop(executable: &Path) -> Result<RunningApp, Failure> {
        tokio::process::Command::new(executable)
            .kill_on_drop(true)
            .group_spawn()
            .map(|child| RunningApp::Desktop(Box::new(child)))
            .map_err(|error| {
                native_tool_failure(
                    1605,
                    &format!("Cannot start the app {}: {error}", executable.display()),
                )
            })
    }

    async fn simulator(build: &NativeBuildResult) -> Result<RunningApp, Failure> {
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
        Ok(RunningApp::Simulator { device, identifier })
    }

    async fn emulator(build: &NativeBuildResult, port: &str) -> Result<RunningApp, Failure> {
        let adb = Self::adb();
        let tool = adb.to_string_lossy().into_owned();
        let forwarded = format!("tcp:{port}");
        let package = String::from(build.launch_identifier());
        let activity = format!("{package}/{}", Self::ANDROID_ACTIVITY);
        let apk = build.application_bundle().to_string_lossy().into_owned();
        run_tool(&tool, &["reverse", &forwarded, &forwarded]).await?;
        run_tool(&tool, &["install", "-r", &apk]).await?;
        run_tool(&tool, &["shell", "am", "start", "-W", "-n", &activity]).await?;
        Ok(RunningApp::Emulator { adb, package })
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
