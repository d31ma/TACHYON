use std::future::{Future, pending};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use tokio::sync::watch;
use tokio::task::LocalKey;

use super::startup_activity::StartupActivity;

pub(crate) struct StoreCall {
    deadline: watch::Sender<Instant>,
    startup_left: Mutex<Duration>,
    waited: Mutex<Duration>,
    waiting_since: Mutex<Option<Instant>>,
    started_up: Mutex<Duration>,
    extended: Mutex<Duration>,
    activity: Arc<StartupActivity>,
    startup_open: Mutex<bool>,
}

impl StoreCall {
    const UNMEASURED: Duration = Duration::from_hours(24);

    pub(super) fn new(
        deadline: Instant,
        startup_allowance: Duration,
        activity: Arc<StartupActivity>,
    ) -> Arc<Self> {
        Arc::new(Self {
            deadline: watch::Sender::new(deadline),
            startup_left: Mutex::new(startup_allowance),
            waited: Mutex::new(Duration::ZERO),
            waiting_since: Mutex::new(None),
            started_up: Mutex::new(Duration::ZERO),
            extended: Mutex::new(Duration::ZERO),
            activity,
            startup_open: Mutex::new(false),
        })
    }

    pub(super) async fn run<Work: Future>(self: &Arc<Self>, work: Work) -> Option<Work::Output> {
        let scoped = Self::key().scope(Arc::clone(self), work);
        let output = self.before_deadline(scoped).await;
        self.end_wait();
        self.end_open_startup();
        output
    }

    pub(crate) fn waited(&self) -> Duration {
        *Self::locked(&self.waited)
    }

    pub(crate) fn started_up(&self) -> Duration {
        *Self::locked(&self.started_up)
    }

    pub(crate) fn extended(&self) -> Duration {
        *Self::locked(&self.extended)
    }

    pub(super) async fn waiting<Work: Future>(work: Work) -> Work::Output {
        Self::with_current(Self::begin_wait);
        let output = work.await;
        Self::with_current(Self::end_wait);
        output
    }

    pub(crate) async fn waiting_behind<Work: Future>(work: Work) -> Work::Output {
        let current = Self::key().try_with(Arc::clone).ok();
        let call = current.unwrap_or_else(Self::unmeasured);
        call.wait_behind(work).await
    }

    pub(crate) async fn startup<Work: Future>(work: Work) -> Work::Output {
        let started = Instant::now();
        let deadline = Self::with_current(Self::open_startup);
        let output = work.await;
        let took = started.elapsed();
        Self::close_open_startup(deadline, took);
        output
    }

    fn close_open_startup(deadline: Option<Instant>, took: Duration) {
        if let Some(before) = deadline {
            Self::with_current(|call| call.close_startup(before, took));
        }
    }

    fn unmeasured() -> Arc<Self> {
        let deadline = Instant::now() + Self::UNMEASURED;
        Self::new(deadline, Duration::ZERO, Arc::new(StartupActivity::new()))
    }

    async fn before_deadline<Work: Future>(&self, work: Work) -> Option<Work::Output> {
        tokio::select! {
            biased;
            output = work => Some(output),
            () = self.expired() => None,
        }
    }

    async fn expired(&self) {
        let mut changes = self.deadline.subscribe();
        while !Self::passed(&mut changes) {
            Self::sleep_or_change(&mut changes).await;
        }
    }

    fn passed(changes: &mut watch::Receiver<Instant>) -> bool {
        Instant::now() >= *changes.borrow_and_update()
    }

    async fn sleep_or_change(changes: &mut watch::Receiver<Instant>) {
        let deadline = *changes.borrow();
        tokio::select! {
            () = tokio::time::sleep_until(deadline.into()) => {}
            _sender_kept = changes.changed() => {}
        }
    }

    async fn wait_behind<Work: Future>(&self, work: Work) -> Work::Output {
        self.begin_wait();
        let before = *self.deadline.borrow();
        let spent_before = self.activity.spent_now();
        let following = self.follow_startups_until_done(before, spent_before);
        let output = Self::first_of(work, following).await;
        let behind = self.activity.spent_now().saturating_sub(spent_before);
        let extra = self.spend_allowance(behind);
        self.deadline.send_replace(before + extra);
        *Self::locked(&self.extended) += extra;
        self.end_wait();
        output
    }

    async fn first_of<Output>(
        work: impl Future<Output = Output>,
        other: impl Future<Output = Output>,
    ) -> Output {
        tokio::select! {
            output = work => output,
            output = other => output,
        }
    }

    async fn follow_startups_until_done<Output>(
        &self,
        before: Instant,
        spent_before: Duration,
    ) -> Output {
        let mut changes = self.activity.subscribe();
        self.follow_startups(before, spent_before);
        while changes.changed().await.is_ok() {
            self.follow_startups(before, spent_before);
        }
        pending().await
    }

    fn follow_startups(&self, before: Instant, spent_before: Duration) {
        let left = *Self::locked(&self.startup_left);
        let behind = self.activity.spent_now().saturating_sub(spent_before);
        let room = [behind.min(left), left][usize::from(self.activity.is_running())];
        self.deadline.send_replace(before + room);
    }

    fn open_startup(&self) -> Instant {
        self.begin_wait();
        self.activity.begin();
        *Self::locked(&self.startup_open) = true;
        let before = *self.deadline.borrow();
        let allowance = *Self::locked(&self.startup_left);
        self.deadline.send_replace(before + allowance);
        before
    }

    fn close_startup(&self, before: Instant, took: Duration) {
        let extra = self.spend_allowance(took);
        self.deadline.send_replace(before + extra);
        *Self::locked(&self.started_up) += took;
        *Self::locked(&self.extended) += extra;
        self.end_wait();
        self.end_open_startup();
    }

    fn end_open_startup(&self) {
        let open = std::mem::take(&mut *Self::locked(&self.startup_open));
        if open {
            self.activity.end();
        }
    }

    fn spend_allowance(&self, took: Duration) -> Duration {
        let mut left = Self::locked(&self.startup_left);
        let extra = took.min(*left);
        *left -= extra;
        extra
    }

    fn begin_wait(&self) {
        *Self::locked(&self.waiting_since) = Some(Instant::now());
    }

    fn end_wait(&self) {
        let since = Self::locked(&self.waiting_since).take();
        *Self::locked(&self.waited) += since.map_or(Duration::ZERO, |since| since.elapsed());
    }

    fn with_current<Answer>(action: impl FnOnce(&Self) -> Answer) -> Option<Answer> {
        Self::key().try_with(|call| action(call)).ok()
    }

    fn locked<Value>(value: &Mutex<Value>) -> MutexGuard<'_, Value> {
        value.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn key() -> &'static LocalKey<Arc<StoreCall>> {
        tokio::task_local! {
            static CURRENT: Arc<StoreCall>;
        }
        &CURRENT
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use super::{StartupActivity, StoreCall};

    fn call(deadline_ms: u64, startup_ms: u64) -> Arc<StoreCall> {
        call_in(deadline_ms, startup_ms, &Arc::new(StartupActivity::new()))
    }

    fn call_in(
        deadline_ms: u64,
        startup_ms: u64,
        activity: &Arc<StartupActivity>,
    ) -> Arc<StoreCall> {
        StoreCall::new(
            Instant::now() + Duration::from_millis(deadline_ms),
            Duration::from_millis(startup_ms),
            Arc::clone(activity),
        )
    }

    async fn pause(milliseconds: u64) {
        tokio::time::sleep(Duration::from_millis(milliseconds)).await;
    }

    #[tokio::test]
    async fn work_that_passes_the_deadline_is_dropped() {
        let call = call(100, 1_000);
        let answer = call.run(pause(400)).await;
        assert_eq!(answer, None);
    }

    #[tokio::test]
    async fn a_startup_request_may_run_past_the_call_deadline() {
        let call = call(100, 1_000);
        let answer = call.run(StoreCall::startup(pause(300))).await;
        assert_eq!(answer, Some(()));
        assert!(call.started_up() >= Duration::from_millis(300));
    }

    #[tokio::test]
    async fn a_startup_request_still_has_its_own_limit() {
        let call = call(100, 200);
        let started = Instant::now();
        let answer = call.run(StoreCall::startup(pause(2_000))).await;
        assert_eq!(answer, None);
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[tokio::test]
    async fn the_extra_time_is_capped_by_the_allowance() {
        let call = call(100, 200);
        let work = async {
            StoreCall::startup(pause(250)).await;
            pause(150).await;
        };
        let started = Instant::now();
        assert_eq!(call.run(work).await, None);
        assert!(started.elapsed() < Duration::from_millis(380));
        assert_eq!(call.extended(), Duration::from_millis(200));
        assert!(call.started_up() >= Duration::from_millis(250));
    }

    #[tokio::test]
    async fn a_second_startup_request_gets_only_what_is_left() {
        let call = call(100, 300);
        let work = async {
            StoreCall::startup(pause(200)).await;
            StoreCall::startup(pause(300)).await;
        };
        assert_eq!(call.run(work).await, None);
        assert!(call.extended() >= Duration::from_millis(200));
        assert!(call.extended() < Duration::from_millis(300));
    }

    #[tokio::test]
    async fn the_time_after_a_startup_request_keeps_the_rest_of_the_deadline() {
        let in_time = async {
            StoreCall::startup(pause(500)).await;
            pause(100).await;
        };
        assert_eq!(call(300, 1_000).run(in_time).await, Some(()));
        let too_late = async {
            StoreCall::startup(pause(500)).await;
            pause(500).await;
        };
        let started = Instant::now();
        assert_eq!(call(300, 1_000).run(too_late).await, None);
        assert!(started.elapsed() < Duration::from_millis(950));
    }

    #[tokio::test]
    async fn waiting_and_startup_time_count_as_wait() {
        let call = call(2_000, 1_000);
        let work = async {
            StoreCall::waiting(pause(50)).await;
            StoreCall::startup(pause(50)).await;
            pause(50).await;
        };
        call.run(work).await;
        assert!(call.waited() >= Duration::from_millis(100));
        assert!(call.waited() < Duration::from_millis(150));
        assert!(call.started_up() >= Duration::from_millis(50));
    }

    #[tokio::test]
    async fn a_wait_cut_off_by_the_deadline_still_counts_as_wait() {
        let call = call(150, 1_000);
        let answer = call.run(StoreCall::waiting(pause(2_000))).await;
        assert_eq!(answer, None);
        assert!(call.waited() >= Duration::from_millis(150));
        assert!(call.waited() < Duration::from_secs(1));
    }

    #[tokio::test]
    async fn a_call_waiting_behind_another_calls_startup_keeps_its_time() {
        let activity = Arc::new(StartupActivity::new());
        let process = tokio::sync::Mutex::new(());
        let first = call_in(100, 1_000, &activity);
        let second = call_in(100, 1_000, &activity);
        let slow_startup = first.run(async {
            let _held = process.lock().await;
            StoreCall::startup(pause(400)).await;
        });
        let waiting = second.run(async {
            pause(20).await;
            let _held = StoreCall::waiting_behind(process.lock()).await;
            pause(20).await;
        });
        let (first_answer, second_answer) = tokio::join!(slow_startup, waiting);
        assert_eq!(first_answer, Some(()));
        assert_eq!(second_answer, Some(()));
        assert!(second.extended() >= Duration::from_millis(300));
        assert!(second.extended() <= Duration::from_millis(400));
    }

    #[tokio::test]
    async fn waiting_behind_a_slow_call_that_is_not_a_startup_still_times_out() {
        let activity = Arc::new(StartupActivity::new());
        let process = tokio::sync::Mutex::new(());
        let first = call_in(2_000, 1_000, &activity);
        let second = call_in(100, 1_000, &activity);
        let slow = first.run(async {
            let _held = process.lock().await;
            pause(400).await;
        });
        let waiting = second.run(async {
            pause(20).await;
            let _held = StoreCall::waiting_behind(process.lock()).await;
        });
        let started = Instant::now();
        let (_, second_answer) = tokio::join!(slow, waiting);
        assert_eq!(second_answer, None);
        assert!(started.elapsed() >= Duration::from_millis(390));
        assert_eq!(second.extended(), Duration::ZERO);
    }

    #[tokio::test]
    async fn a_startup_cut_off_by_the_deadline_does_not_stay_running() {
        let activity = Arc::new(StartupActivity::new());
        let call = call_in(50, 100, &activity);
        let answer = call.run(StoreCall::startup(pause(1_000))).await;
        assert_eq!(answer, None);
        assert!(!activity.is_running());
    }

    #[tokio::test]
    async fn work_outside_a_store_call_runs_unmeasured() {
        StoreCall::startup(pause(1)).await;
        StoreCall::waiting(pause(1)).await;
    }
}
