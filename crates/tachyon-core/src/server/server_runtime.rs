mod background_plan;
mod producer_task_state;
mod producer_tasks;

pub(in crate::server) use background_plan::BackgroundPlan;
pub(in crate::server) use producer_tasks::ProducerTasks;

use super::WatchTask;
use super::WorkerRunner;
use crate::handler::HandlerSupervisor;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::{JoinError, JoinSet};
use tokio::time::{Instant, Interval};
use tokio_util::sync::CancellationToken;

pub(in crate::server) struct ServerRuntime {
    pub(in crate::server) supervisor: Arc<HandlerSupervisor>,
    pub(in crate::server) cancellation: CancellationToken,
    pub(in crate::server) tasks: JoinSet<()>,
    pub(in crate::server) producers: ProducerTasks,
    pub(in crate::server) plan: Option<BackgroundPlan>,
}

impl std::fmt::Debug for ServerRuntime {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ServerRuntime")
            .field("started", &self.started())
            .field("tasks", &self.tasks.len())
            .finish_non_exhaustive()
    }
}

impl Drop for ServerRuntime {
    fn drop(&mut self) {
        self.cancellation.cancel();
        self.tasks.abort_all();
        self.producers.close_and_abort();
    }
}

impl ServerRuntime {
    pub(in crate::server) const SHUTDOWN: Duration = Duration::from_secs(3);
    pub(in crate::server) const ABORT_SETTLEMENT: Duration = Duration::from_millis(500);
    const COOPERATIVE_TASK_SETTLEMENT: Duration = Duration::from_secs(1);
    pub(in crate::server) const WORKER_CANCELLATION_SETTLE: Duration = Duration::from_secs(1);
    const YIELD_EVERY: usize = 16;

    pub(in crate::server) fn new(plan: BackgroundPlan) -> Self {
        Self {
            supervisor: Arc::clone(&plan.supervisor),
            cancellation: CancellationToken::new(),
            tasks: JoinSet::new(),
            producers: ProducerTasks::new(),
            plan: Some(plan),
        }
    }

    pub(in crate::server) fn started(&self) -> bool {
        self.plan.is_none()
    }

    pub(in crate::server) fn start(&mut self) {
        let plan = self.plan.take();
        self.start_when_planned(plan);
    }

    pub(in crate::server) async fn shutdown_until(&mut self, hard_deadline: Instant) {
        self.cancellation.cancel();
        let mut background = std::mem::take(&mut self.tasks);
        let mut producers = self.producers.take();
        let cooperative_deadline = Self::cooperative_shutdown_deadline(hard_deadline)
            .min(Instant::now() + Self::COOPERATIVE_TASK_SETTLEMENT);
        let drained = tokio::time::timeout_at(
            cooperative_deadline,
            Self::drain_tasks(&mut background, &mut producers),
        )
        .await
        .is_ok();
        Self::abort_unless_drained(drained, &mut background, &mut producers, hard_deadline).await;
        let _settled =
            tokio::time::timeout_at(hard_deadline, self.supervisor.close_resources()).await;
    }

    pub(in crate::server) fn cooperative_shutdown_deadline(hard_deadline: Instant) -> Instant {
        hard_deadline
            .checked_sub(Self::ABORT_SETTLEMENT)
            .unwrap_or(hard_deadline)
    }

    pub(in crate::server) async fn reap_producer_tasks(
        producers: ProducerTasks,
        cancellation: CancellationToken,
    ) {
        let mut interval = tokio::time::interval(ProducerTasks::REAP_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        Self::reap_until_cancelled(&producers, &cancellation, &mut interval).await;
    }

    fn report_task(kind: &str, result: Result<(), JoinError>) {
        if let Err(error) = result
            && !error.is_cancelled()
        {
            eprintln!("Development {kind} task failed: {error}");
        }
    }

    fn start_when_planned(&mut self, plan: Option<BackgroundPlan>) {
        if let Some(plan) = plan {
            self.spawn_background(plan);
        }
    }

    fn spawn_background(&mut self, plan: BackgroundPlan) {
        self.spawn_workers(&plan.workers, &plan.supervisor);
        self.tasks.spawn(
            plan.topics
                .run(Arc::clone(&plan.supervisor), self.cancellation.clone()),
        );
        self.spawn_watcher(plan.watcher);
        let producers = self.producers.clone();
        let cancellation = self.cancellation.clone();
        self.tasks
            .spawn(Self::reap_producer_tasks(producers, cancellation));
    }

    fn spawn_workers(
        &mut self,
        workers: &[crate::project::ScheduledWorker],
        supervisor: &Arc<HandlerSupervisor>,
    ) {
        for worker in workers {
            WorkerRunner::spawn(worker, supervisor, &self.cancellation, &mut self.tasks);
        }
    }

    fn spawn_watcher(&mut self, watcher: Option<WatchTask>) {
        if let Some(watch) = watcher {
            let cancellation = self.cancellation.clone();
            self.tasks.spawn(watch.run(cancellation));
        }
    }

    async fn abort_unless_drained(
        drained: bool,
        background: &mut JoinSet<()>,
        producers: &mut JoinSet<()>,
        hard_deadline: Instant,
    ) {
        if !drained {
            Self::abort_tasks(background, producers, hard_deadline).await;
        }
    }

    async fn abort_tasks(
        background: &mut JoinSet<()>,
        producers: &mut JoinSet<()>,
        hard_deadline: Instant,
    ) {
        background.abort_all();
        producers.abort_all();
        let drained =
            tokio::time::timeout_at(hard_deadline, Self::drain_tasks(background, producers))
                .await
                .is_ok();
        Self::abort_again_unless_drained(drained, background, producers);
    }

    fn abort_again_unless_drained(
        drained: bool,
        background: &mut JoinSet<()>,
        producers: &mut JoinSet<()>,
    ) {
        if drained {
            return;
        }
        background.abort_all();
        producers.abort_all();
        eprintln!("Development server tasks did not settle within the abort deadline.");
    }

    async fn drain_tasks(background: &mut JoinSet<()>, producers: &mut JoinSet<()>) {
        let mut joined = 0_usize;
        while !background.is_empty() || !producers.is_empty() {
            joined = Self::join_one(background, producers, joined).await;
        }
    }

    async fn join_one(
        background: &mut JoinSet<()>,
        producers: &mut JoinSet<()>,
        joined: usize,
    ) -> usize {
        let (kind, result) = Self::next_finished(background, producers).await;
        Self::count_joined(kind, result, joined).await
    }

    async fn next_finished(
        background: &mut JoinSet<()>,
        producers: &mut JoinSet<()>,
    ) -> (&'static str, Option<Result<(), JoinError>>) {
        tokio::select! {
            result = background.join_next(), if !background.is_empty() => ("background", result),
            result = producers.join_next(), if !producers.is_empty() => ("response producer", result),
        }
    }

    async fn count_joined(
        kind: &str,
        result: Option<Result<(), JoinError>>,
        joined: usize,
    ) -> usize {
        match result {
            None => joined,
            Some(result) => Self::record_joined(kind, result, joined).await,
        }
    }

    async fn record_joined(kind: &str, result: Result<(), JoinError>, joined: usize) -> usize {
        Self::report_task(kind, result);
        let joined = joined + 1;
        Self::yield_every_few(joined).await;
        joined
    }

    async fn yield_every_few(joined: usize) {
        if joined.is_multiple_of(Self::YIELD_EVERY) {
            tokio::task::yield_now().await;
        }
    }

    async fn reap_until_cancelled(
        producers: &ProducerTasks,
        cancellation: &CancellationToken,
        interval: &mut Interval,
    ) {
        while Self::reap_on_tick(producers, cancellation, interval).await {}
    }

    async fn reap_on_tick(
        producers: &ProducerTasks,
        cancellation: &CancellationToken,
        interval: &mut Interval,
    ) -> bool {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => {
                producers.reap_finished();
                false
            }
            _ = interval.tick() => {
                producers.reap_finished();
                true
            }
        }
    }
}
