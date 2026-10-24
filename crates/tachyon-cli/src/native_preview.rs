use crate::ServingPolicy;
use crate::advice_report::AdviceReport;
use crate::resilient_serve::ResilientServe;
use crate::shutdown::LongRunningCommand;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tachyon_contracts::NativeTarget;
use tachyon_core::{
    DevelopmentOrigin, Failure, NativeAppLaunch, NativeAppSources, NativeBuildOptions,
    NativeBuildResult, NativeCompiler,
};
use tokio::sync::watch;

pub(crate) struct NativePreview {
    pub(crate) project: PathBuf,
    pub(crate) host: Option<String>,
    pub(crate) port: Option<u16>,
    pub(crate) allow_non_loopback: bool,
    pub(crate) target: NativeTarget,
    pub(crate) advice: AdviceReport,
}

impl NativePreview {
    const DEFAULT_PORT: u16 = 3000;
    const APP_OUTPUT: &'static str = "dist/preview";
    const PAGE_OUTPUT: &'static str = "dist/preview/pages";
    const POLL: Duration = Duration::from_millis(500);
    const SERVER_WAIT: Duration = Duration::from_secs(300);

    pub(crate) async fn run(self) -> Result<(), Failure> {
        let development = self.development_origin()?;
        let (stop, stopped) = watch::channel(false);
        let app = tokio::spawn(AppKeeper::keep_running(
            self.project.clone(),
            development.clone(),
            stopped,
        ));
        let served = self.serve(development).await;
        stop.send_replace(true);
        let _kept = app.await;
        served
    }

    async fn serve(self, development: DevelopmentOrigin) -> Result<(), Failure> {
        ResilientServe {
            project: self.project,
            host: self.host,
            port: self.port,
            allow_non_loopback: self.allow_non_loopback,
            output: PathBuf::from(Self::PAGE_OUTPUT)
                .join(tachyon_core::native_target_directory(self.target)),
            policy: ServingPolicy {
                watch: true,
                build: true,
                self_restart: false,
                command: LongRunningCommand::Preview,
            },
            default_port: Self::DEFAULT_PORT,
            advice: self.advice,
            native: Some(development),
        }
        .run()
        .await
    }

    fn development_origin(&self) -> Result<DevelopmentOrigin, Failure> {
        let application = crate::application_for_server(
            &self.project,
            self.host.as_deref(),
            self.port,
            self.allow_non_loopback,
        )
        .ok()
        .map(|(_, application)| application);
        let address = application
            .as_ref()
            .and_then(tachyon_core::ApplicationConfig::host)
            .unwrap_or(IpAddr::from([127, 0, 0, 1]));
        let port = application
            .as_ref()
            .and_then(tachyon_core::ApplicationConfig::port)
            .or(self.port)
            .unwrap_or(Self::DEFAULT_PORT);
        if port == 0 {
            return Err(crate::cli_failure(
                1301,
                "The native preview needs a fixed port, because the app is built to reach it.",
                "Pass --port with a free port number, such as --port 3000.",
            ));
        }
        if !(address.is_loopback() || address.is_unspecified()) {
            return Err(crate::cli_failure(
                1301,
                "The native preview app reaches the development server on this machine's loopback address.",
                "Use a loopback host such as 127.0.0.1, or leave out --host.",
            ));
        }
        Ok(DevelopmentOrigin::new(self.target, port))
    }
}

struct AppKeeper {
    project: PathBuf,
    development: DevelopmentOrigin,
    stopped: watch::Receiver<bool>,
}

impl AppKeeper {
    async fn keep_running(
        project: PathBuf,
        development: DevelopmentOrigin,
        stopped: watch::Receiver<bool>,
    ) {
        let mut keeper = Self {
            project,
            development,
            stopped,
        };
        if !keeper.wait_for_server().await {
            return;
        }
        let mut running = keeper.launch().await;
        let mut sources = keeper.sources();
        let mut exit_reported = false;
        while keeper.pause().await {
            let exited = running.as_mut().is_some_and(NativeAppLaunch::has_exited);
            if exited && !exit_reported {
                println!(
                    "The {} app closed. ty preview keeps serving; save a source file to start it again.",
                    keeper.target_name()
                );
            }
            exit_reported |= exited;
            let next = keeper.sources();
            if next == sources {
                continue;
            }
            sources = next;
            if let Some(app) = running.take() {
                app.stop().await;
            }
            println!(
                "Rebuilding the {} app because its manifest or native sources changed.",
                keeper.target_name()
            );
            running = keeper.launch().await;
            exit_reported = false;
        }
        if let Some(app) = running {
            app.stop().await;
        }
    }

    fn target_name(&self) -> &'static str {
        tachyon_core::native_target_directory(self.development.target())
    }

    fn sources(&self) -> u64 {
        NativeAppSources::fingerprint(&self.project, self.development.target())
    }

    async fn pause(&mut self) -> bool {
        tokio::select! {
            biased;
            _ = self.stopped.wait_for(|stopped| *stopped) => false,
            () = tokio::time::sleep(NativePreview::POLL) => true,
        }
    }

    async fn wait_for_server(&mut self) -> bool {
        let address = self.server_address();
        let deadline = tokio::time::Instant::now() + NativePreview::SERVER_WAIT;
        while tokio::time::Instant::now() < deadline {
            if tokio::net::TcpStream::connect(&address).await.is_ok() {
                return true;
            }
            if !self.pause().await {
                return false;
            }
        }
        eprintln!("The development server did not start, so ty preview did not open the app.");
        false
    }

    fn server_address(&self) -> String {
        format!("127.0.0.1:{}", self.development.port())
    }

    async fn launch(&self) -> Option<NativeAppLaunch> {
        let build = match self.build().await {
            Ok(build) => build,
            Err(failure) => {
                eprint!("{failure}");
                eprintln!("Fix the problem and save; ty preview builds the app again.");
                return None;
            }
        };
        crate::print_native_build(self.development.target(), &build);
        match NativeAppLaunch::start(&build, &self.development).await {
            Ok(app) => {
                println!(
                    "Opened the {} app against {}; edits reach it as hot updates.",
                    self.target_name(),
                    self.development.origin()
                );
                Some(app)
            }
            Err(failure) => {
                eprint!("{failure}");
                None
            }
        }
    }

    async fn build(&self) -> Result<NativeBuildResult, Failure> {
        NativeCompiler::build_async(
            &self.project,
            &NativeBuildOptions {
                output_directory: Path::new(NativePreview::APP_OUTPUT).to_path_buf(),
                target: self.development.target(),
                development_origin: Some(self.development.clone()),
                ..NativeBuildOptions::default()
            },
        )
        .await
    }
}
