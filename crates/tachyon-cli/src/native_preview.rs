use crate::ServingPolicy;
use crate::advice_report::AdviceReport;
use crate::native_app_keeper::NativeAppKeeper;
use crate::resilient_serve::ResilientServe;
use crate::shutdown::LongRunningCommand;
use std::net::IpAddr;
use std::path::PathBuf;
use tachyon_contracts::NativeTarget;
use tachyon_core::{DevelopmentOrigin, Failure};
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
    const PAGE_OUTPUT: &'static str = "dist/preview/pages";

    pub(crate) async fn run(self) -> Result<(), Failure> {
        let development = self.development_origin()?;
        let (stop, stopped) = watch::channel(false);
        let app = tokio::spawn(NativeAppKeeper::keep_running(
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
        let remote = !address.is_loopback() & !address.is_unspecified();
        [
            (port == 0).then(Self::port_refusal),
            remote.then(Self::host_refusal),
        ]
        .into_iter()
        .flatten()
        .next()
        .map_or_else(|| Ok(DevelopmentOrigin::new(self.target, port)), Err)
    }

    fn port_refusal() -> Failure {
        crate::cli_failure(
            1301,
            "The native preview needs a fixed port, because the app is built to reach it.",
            "Pass --port with a free port number, such as --port 3000.",
        )
    }

    fn host_refusal() -> Failure {
        crate::cli_failure(
            1301,
            "The native preview app reaches the development server on this machine's loopback address.",
            "Use a loopback host such as 127.0.0.1, or leave out --host.",
        )
    }
}
