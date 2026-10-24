use crate::native_build_report::NativeBuildReport;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tachyon_core::{
    DevelopmentOrigin, Failure, NativeAppLaunch, NativeAppSources, NativeBuildOptions,
    NativeBuildResult, NativeCompiler,
};
use tokio::sync::watch;

pub(crate) struct NativeAppKeeper {
    project: PathBuf,
    development: DevelopmentOrigin,
    stopped: watch::Receiver<bool>,
    running: Option<NativeAppLaunch>,
    sources: u64,
    exit_reported: bool,
}

impl NativeAppKeeper {
    const APP_OUTPUT: &'static str = "dist/preview";
    const POLL: Duration = Duration::from_millis(500);
    const SERVER_WAIT: Duration = Duration::from_mins(5);

    pub(crate) async fn keep_running(
        project: PathBuf,
        development: DevelopmentOrigin,
        stopped: watch::Receiver<bool>,
    ) {
        let mut keeper = Self {
            project,
            development,
            stopped,
            running: None,
            sources: 0,
            exit_reported: false,
        };
        let reached = keeper.wait_for_server().await;
        keeper.serve_while(reached).await;
        keeper.close().await;
    }

    async fn serve_while(&mut self, reached: bool) {
        if !reached {
            return;
        }
        self.sources = self.sources();
        self.running = self.launch().await;
        self.watch().await;
    }

    async fn watch(&mut self) {
        while self.pause().await {
            self.report_exit();
            self.relaunch_when_changed().await;
        }
    }

    fn report_exit(&mut self) {
        let exited = self
            .running
            .as_mut()
            .is_some_and(NativeAppLaunch::has_exited);
        if [exited, !self.exit_reported] == [true, true] {
            println!(
                "The {} app closed. ty preview keeps serving; save a source file to start it again.",
                self.target_name()
            );
        }
        self.exit_reported |= exited;
    }

    async fn relaunch_when_changed(&mut self) {
        let next = self.sources();
        if next == self.sources {
            return;
        }
        self.sources = next;
        self.close().await;
        println!(
            "Rebuilding the {} app because its manifest or native sources changed.",
            self.target_name()
        );
        self.running = self.launch().await;
        self.exit_reported = false;
    }

    async fn close(&mut self) {
        while let Some(app) = self.running.take() {
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
        tokio::time::timeout(Self::POLL, self.stopped.wait_for(|stopped| *stopped))
            .await
            .is_err()
    }

    async fn wait_for_server(&mut self) -> bool {
        let deadline = tokio::time::Instant::now() + Self::SERVER_WAIT;
        let mut reached = false;
        while self.unreached(deadline, &mut reached).await {}
        reached
    }

    async fn unreached(&mut self, deadline: tokio::time::Instant, reached: &mut bool) -> bool {
        *reached = tokio::net::TcpStream::connect(self.server_address())
            .await
            .is_ok();
        let late = tokio::time::Instant::now() >= deadline;
        late.then(|| {
            eprintln!("The development server did not start, so ty preview did not open the app.");
        });
        if [*reached, late].contains(&true) {
            return false;
        }
        self.pause().await
    }

    fn server_address(&self) -> String {
        format!("127.0.0.1:{}", self.development.port())
    }

    async fn launch(&self) -> Option<NativeAppLaunch> {
        let build = self
            .build()
            .await
            .map_err(|failure| Self::build_failed(&failure))
            .ok()?;
        NativeBuildReport::print(self.development.target(), &build);
        NativeAppLaunch::start(&build, &self.development)
            .await
            .map(|app| self.opened(app))
            .map_err(|failure| eprint!("{failure}"))
            .ok()
    }

    fn build_failed(failure: &Failure) {
        eprint!("{failure}");
        eprintln!("Fix the problem and save; ty preview builds the app again.");
    }

    fn opened(&self, app: NativeAppLaunch) -> NativeAppLaunch {
        println!(
            "Opened the {} app against {}; edits reach it as hot updates.",
            self.target_name(),
            self.development.origin()
        );
        app
    }

    async fn build(&self) -> Result<NativeBuildResult, Failure> {
        NativeCompiler::build_async(
            &self.project,
            &NativeBuildOptions {
                output_directory: Path::new(Self::APP_OUTPUT).to_path_buf(),
                target: self.development.target(),
                development_origin: Some(self.development.clone()),
                ..NativeBuildOptions::default()
            },
        )
        .await
    }
}
