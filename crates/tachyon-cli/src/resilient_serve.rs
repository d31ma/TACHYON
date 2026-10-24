use crate::advice_report::AdviceReport;
use crate::shutdown::ShutdownSignals;
use crate::start_step::StartStep;
use crate::{ServingPolicy, restart};
use std::io::Write as _;
use std::net::IpAddr;
use std::path::PathBuf;
use tachyon_core::{DevServer, DevServerOptions, ErrorPageServer, ErrorPageWait, Failure};
use tokio::sync::watch;

pub(crate) struct ResilientServe {
    pub(crate) project: PathBuf,
    pub(crate) host: Option<String>,
    pub(crate) port: Option<u16>,
    pub(crate) allow_non_loopback: bool,
    pub(crate) output: PathBuf,
    pub(crate) policy: ServingPolicy,
    pub(crate) default_port: u16,
    pub(crate) advice: AdviceReport,
    pub(crate) native: Option<tachyon_core::DevelopmentOrigin>,
}

impl ResilientServe {
    pub(crate) async fn run(&self) -> Result<(), Failure> {
        let stopped = Self::stop_signal(self.policy);
        let step = self.first_server(&stopped).await?;
        self.serve_step(step, stopped).await
    }

    async fn first_server(&self, stopped: &watch::Receiver<bool>) -> Result<StartStep, Failure> {
        let mut step = StartStep::Retry(None);
        while let StartStep::Retry(error_page) = step {
            step = self
                .next_step(error_page.map(|page| *page), stopped.clone())
                .await?;
        }
        Ok(step)
    }

    async fn next_step(
        &self,
        error_page: Option<ErrorPageServer>,
        stopped: watch::Receiver<bool>,
    ) -> Result<StartStep, Failure> {
        let mut error_page = error_page;
        let started = self.start(&mut error_page).await;
        self.after_start(started, error_page, stopped).await
    }

    async fn after_start(
        &self,
        started: Result<DevServer, Failure>,
        error_page: Option<ErrorPageServer>,
        stopped: watch::Receiver<bool>,
    ) -> Result<StartStep, Failure> {
        match started {
            Ok(server) => Ok(StartStep::Serve(Box::new(server))),
            Err(failure) => self.show_problem(error_page, &failure, stopped).await,
        }
    }

    async fn show_problem(
        &self,
        error_page: Option<ErrorPageServer>,
        failure: &Failure,
        stopped: watch::Receiver<bool>,
    ) -> Result<StartStep, Failure> {
        eprint!("{failure}");
        let mut page = self.error_page(error_page, failure).await?;
        let waited = Self::waited(&mut page, stopped).await;
        Self::after_wait(waited, page).await
    }

    async fn serve_step(
        &self,
        step: StartStep,
        stopped: watch::Receiver<bool>,
    ) -> Result<(), Failure> {
        match step {
            StartStep::Serve(server) => {
                crate::serve_dev_server(*server, self.policy, Self::stopped(stopped)).await
            }
            StartStep::Retry(_) | StartStep::Stopped => Ok(()),
        }
    }

    async fn start(&self, error_page: &mut Option<ErrorPageServer>) -> Result<DevServer, Failure> {
        let (project, application) = crate::application_for_server(
            &self.project,
            self.host.as_deref(),
            self.port,
            self.allow_non_loopback,
        )?;
        let host = application.host().unwrap_or(IpAddr::from([127, 0, 0, 1]));
        let port = application.port().unwrap_or(self.default_port);
        let untested = crate::require_tested_methods(project.root(), self.policy.command)
            .await
            .err();
        Self::stop_page(error_page.take()).await;
        tachyon_core::cache::ensure_runtime()?;
        let options = DevServerOptions {
            native: self.native.clone(),
            ..crate::dev_server_options(
                host,
                port,
                self.allow_non_loopback,
                &self.output,
                self.policy,
            )
        };
        let server =
            DevServer::bind_project_with_application(&project, &options, application).await?;
        Self::show_untested(&server, untested);
        self.advice.start_once();
        Ok(server)
    }

    fn show_untested(server: &DevServer, untested: Option<Failure>) {
        if let Some(failure) = untested {
            eprint!("{failure}");
            server.show_start_diagnostics(&failure);
        }
    }

    async fn error_page(
        &self,
        running: Option<ErrorPageServer>,
        failure: &Failure,
    ) -> Result<ErrorPageServer, Failure> {
        let page = match running {
            Some(page) => page,
            None => self.bind_error_page().await?,
        };
        page.show(failure);
        Ok(page)
    }

    async fn bind_error_page(&self) -> Result<ErrorPageServer, Failure> {
        let host = crate::resolve_host(self.host.as_deref(), &[])?;
        let port = self.port.unwrap_or(self.default_port);
        let options = crate::dev_server_options(
            host.address(),
            port,
            self.allow_non_loopback,
            &self.output,
            self.policy,
        );
        let page = ErrorPageServer::bind(&self.project, &options).await?;
        println!(
            "{} at {}",
            crate::ready_label(self.policy.command),
            host.url(page.address())
        );
        println!("Showing the project's problems until it builds; watching sources.");
        let _flush_result = std::io::stdout().flush();
        self.advice.start_once();
        Ok(page)
    }

    async fn waited(
        page: &mut ErrorPageServer,
        stopped: watch::Receiver<bool>,
    ) -> Option<ErrorPageWait> {
        tokio::select! {
            waited = page.wait_for_change() => Some(waited),
            () = Self::stopped(stopped) => None,
        }
    }

    async fn after_wait(
        waited: Option<ErrorPageWait>,
        page: ErrorPageServer,
    ) -> Result<StartStep, Failure> {
        match waited {
            Some(ErrorPageWait::SourcesChanged) => Ok(StartStep::Retry(Some(Box::new(page)))),
            Some(ErrorPageWait::ToolUpdated(tool_restart)) => {
                page.stop().await;
                Err(restart::SelfRestart::replace_process(&tool_restart))
            }
            None => {
                page.stop().await;
                Ok(StartStep::Stopped)
            }
        }
    }

    async fn stop_page(page: Option<ErrorPageServer>) {
        if let Some(page) = page {
            page.stop().await;
        }
    }

    fn stop_signal(policy: ServingPolicy) -> watch::Receiver<bool> {
        let (sender, stopped) = watch::channel(false);
        let signals = ShutdownSignals::install(policy.command);
        tokio::spawn(async move {
            signals.wait().await;
            let _sent = sender.send(true);
        });
        stopped
    }

    async fn stopped(mut stopped: watch::Receiver<bool>) {
        let _stopped_or_closed = stopped.wait_for(|value| *value).await;
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::ResilientServe;
    use crate::ServingPolicy;
    use crate::advice_report::AdviceReport;
    use crate::shutdown::LongRunningCommand;
    use std::io::{Read as _, Write as _};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};

    fn page_answers(address: &str) -> bool {
        let Ok(mut stream) = std::net::TcpStream::connect(address) else {
            return false;
        };
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
        let _ = stream.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
        let mut response = String::new();
        let _ = stream.read_to_string(&mut response);
        response.contains("Slow advice")
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_preview_listens_while_a_slow_advice_pass_still_runs() {
        let project = tempfile::tempdir().expect("project");
        for (relative, source) in [
            ("client/pages/tac.html", "<app-view />"),
            (
                "client/components/app/view/tac.html",
                "<main aria-label=\"Slow\"><h1>Slow advice</h1></main>",
            ),
        ] {
            let path = project.path().join(relative);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("folders");
            std::fs::write(path, source).expect("source");
        }
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .and_then(|listener| listener.local_addr())
            .expect("free port")
            .port();
        let advised = Arc::new(AtomicBool::new(false));
        let finished = Arc::clone(&advised);
        let serve = ResilientServe {
            project: project.path().to_path_buf(),
            host: None,
            port: Some(port),
            allow_non_loopback: false,
            output: std::path::PathBuf::from("dist/web"),
            policy: ServingPolicy {
                watch: true,
                build: true,
                self_restart: false,
                command: LongRunningCommand::Serve,
            },
            default_port: port,
            advice: AdviceReport::new(Box::new(move || {
                std::thread::sleep(Duration::from_mins(1));
                finished.store(true, Ordering::Release);
            })),
            native: None,
        };
        let running = tokio::spawn(async move { serve.run().await });
        let address = format!("127.0.0.1:{port}");
        let deadline = Instant::now() + Duration::from_secs(45);
        while !page_answers(&address) {
            assert!(Instant::now() < deadline, "the preview never answered");
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        assert!(
            !advised.load(Ordering::Acquire),
            "serving waited for advice"
        );
        running.abort();
    }
}
