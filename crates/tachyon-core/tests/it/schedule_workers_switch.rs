#![allow(clippy::expect_used)]

use crate::support::program_probe::ProgramProbe;
use std::path::Path;
use std::time::Duration;
use tachyon_core::{DevServer, DevServerOptions, ProjectDiscovery};

struct WorkersSwitch;

impl WorkersSwitch {
    const WORKER: &'static str = "import { appendFileSync } from 'node:fs'\n\n@Schedule({\"fixedDelaySeconds\":1,\"initialDelaySeconds\":0})\nexport class Handler {\n  static POST() {\n    appendFileSync('__TICKS__', 'tick\\n')\n  }\n}\n";
    const FIRST_TICK_LIMIT: Duration = Duration::from_mins(2);
    const QUIET_PERIOD: Duration = Duration::from_secs(5);
    const POLL: Duration = Duration::from_millis(50);

    fn write(root: &Path, relative: &str, source: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directories");
        std::fs::write(path, source).expect("source");
    }

    fn node_missing() -> bool {
        if ProgramProbe::answers("node", "--version") {
            return false;
        }
        assert!(std::env::var_os("CI").is_none(), "node is required in CI");
        eprintln!("skipping the workers switch: node is unavailable");
        true
    }

    async fn wait_for_tick(ticks: &Path) {
        while !ticks.exists() {
            tokio::time::sleep(Self::POLL).await;
        }
    }

    async fn ticked_within(application: &str, wait: Duration) -> bool {
        let root = tempfile::tempdir().expect("project");
        let ticks_folder = tempfile::tempdir().expect("ticks folder");
        let ticks = ticks_folder.path().join("ticks");
        let worker = Self::WORKER.replace("__TICKS__", &ticks.to_string_lossy().replace('\\', "/"));
        Self::write(root.path(), "server/workers/reconcile.js", &worker);
        Self::write(root.path(), "server/application.json", application);
        Self::write(root.path(), "client/pages/tac.html", "<worker-view />");
        Self::write(
            root.path(),
            "client/components/worker/view/tac.html",
            "<main aria-label=\"Workers\"><h1>Workers</h1></main>",
        );
        let project = ProjectDiscovery::discover(root.path()).expect("project discovery");
        let server = DevServer::bind_project(
            &project,
            &DevServerOptions {
                port: 0,
                watch: false,
                ..DevServerOptions::default()
            },
        )
        .await
        .expect("server binds");
        let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
        let running = tokio::spawn(server.run_until(async {
            let _ = stopped.await;
        }));
        let ticked = tokio::time::timeout(wait, Self::wait_for_tick(&ticks))
            .await
            .is_ok();
        stop.send(()).expect("stop");
        running.await.expect("task").expect("shutdown");
        ticked
    }

    async fn assert_runs(application: &str) {
        if Self::node_missing() {
            return;
        }
        assert!(
            Self::ticked_within(application, Self::FIRST_TICK_LIMIT).await,
            "{application}: the @Schedule worker should run"
        );
    }

    async fn assert_stays_quiet(application: &str) {
        if Self::node_missing() {
            return;
        }
        assert!(
            !Self::ticked_within(application, Self::QUIET_PERIOD).await,
            "{application}: the @Schedule worker should not run"
        );
    }
}

#[tokio::test]
async fn ty_start_runs_schedule_workers_by_default() {
    WorkersSwitch::assert_runs("{}").await;
}

#[tokio::test]
async fn ty_start_runs_schedule_workers_when_workers_is_true() {
    WorkersSwitch::assert_runs(r#"{"server":{"workers":true}}"#).await;
}

#[tokio::test]
async fn ty_start_runs_no_schedule_worker_when_workers_is_false() {
    WorkersSwitch::assert_stays_quiet(r#"{"server":{"workers":false}}"#).await;
}
