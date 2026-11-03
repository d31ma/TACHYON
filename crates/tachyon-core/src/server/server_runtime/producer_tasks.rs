use super::ServerRuntime;
use super::producer_task_state::ProducerTaskState;
use std::future::Future;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;
use tokio::task::JoinSet;

#[derive(Clone)]
pub(in crate::server) struct ProducerTasks {
    inner: Arc<Mutex<ProducerTaskState>>,
}

impl std::fmt::Debug for ProducerTasks {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = self.state();
        formatter
            .debug_struct("ProducerTasks")
            .field("accepting", &state.accepting)
            .field("tasks", &state.tasks.len())
            .finish()
    }
}

impl ProducerTasks {
    pub(in crate::server) const REAP_INTERVAL: Duration = Duration::from_millis(25);

    pub(in crate::server) fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(ProducerTaskState {
                accepting: true,
                tasks: JoinSet::new(),
            })),
        }
    }

    pub(in crate::server) fn spawn<F>(&self, task: F) -> bool
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let mut state = self.state();
        Self::reap(&mut state.tasks);
        Self::spawn_when_accepting(&mut state, task)
    }

    pub(in crate::server) fn close(&self) {
        self.state().accepting = false;
    }

    pub(in crate::server) fn take(&self) -> JoinSet<()> {
        let mut state = self.state();
        state.accepting = false;
        std::mem::take(&mut state.tasks)
    }

    pub(in crate::server) fn close_and_abort(&self) {
        let mut state = self.state();
        state.accepting = false;
        state.tasks.abort_all();
    }

    pub(in crate::server) fn reap_finished(&self) {
        Self::reap(&mut self.state().tasks);
    }

    #[cfg(test)]
    pub(in crate::server) fn len(&self) -> usize {
        self.state().tasks.len()
    }

    fn state(&self) -> MutexGuard<'_, ProducerTaskState> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn spawn_when_accepting<F>(state: &mut ProducerTaskState, task: F) -> bool
    where
        F: Future<Output = ()> + Send + 'static,
    {
        if !state.accepting {
            return false;
        }
        state.tasks.spawn(task);
        true
    }

    fn reap(tasks: &mut JoinSet<()>) {
        while let Some(result) = tasks.try_join_next() {
            ServerRuntime::report_task("response producer", result);
        }
    }
}
