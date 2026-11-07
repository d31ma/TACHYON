use super::{WarmKey, WarmProcess};
use crate::handler::WarmSettings;
use std::cmp::Reverse;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError, Weak};
use std::time::Duration;
use tokio::time::{Instant, sleep};

pub(super) struct WarmPool {
    settings: WarmSettings,
    capacity: usize,
    idle: Mutex<Vec<WarmProcess>>,
    reaper: OnceLock<()>,
    closed: AtomicBool,
}

impl fmt::Debug for WarmPool {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WarmPool")
            .field("settings", &self.settings)
            .field("capacity", &self.capacity)
            .finish_non_exhaustive()
    }
}

impl WarmPool {
    const REAP_EVERY: Duration = Duration::from_secs(1);

    pub(super) fn new(settings: WarmSettings, capacity: usize) -> Self {
        Self {
            settings,
            capacity,
            idle: Mutex::new(Vec::new()),
            reaper: OnceLock::new(),
            closed: AtomicBool::new(false),
        }
    }

    pub(super) async fn take_ready(&self, key: &WarmKey) -> Option<WarmProcess> {
        self.first_ready(key).await
    }

    pub(super) async fn give_back(self: &Arc<Self>, process: Option<WarmProcess>) {
        self.start_reaper();
        let unwanted = process.map(|process| self.keep_or_refuse(process));
        Self::retire_all(unwanted.unwrap_or_default()).await;
    }

    pub(super) async fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
        let idle = std::mem::take(&mut *self.lock());
        Self::retire_all(idle).await;
    }

    async fn first_ready(&self, key: &WarmKey) -> Option<WarmProcess> {
        let mut ready = None;
        while ready.is_none() {
            ready = self.take(key)?.into_ready().await;
        }
        ready
    }

    fn take(&self, key: &WarmKey) -> Option<WarmProcess> {
        let mut idle = self.lock();
        let position = idle.iter().position(|process| process.serves(key))?;
        Some(idle.remove(position))
    }

    fn keep_or_refuse(&self, process: WarmProcess) -> Vec<WarmProcess> {
        let closed = self.closed.load(Ordering::SeqCst);
        let worn_out = process.has_served(self.settings.max_requests);
        if [closed, worn_out].contains(&true) {
            return vec![process];
        }
        let mut idle = self.lock();
        idle.push(process);
        Self::over_capacity(&mut idle, self.capacity)
    }

    fn over_capacity(idle: &mut Vec<WarmProcess>, capacity: usize) -> Vec<WarmProcess> {
        idle.sort_by_key(|process| Reverse(process.idle_since()));
        let kept = idle.len().min(capacity);
        idle.split_off(kept)
    }

    fn take_expired(&self, now: Instant) -> Vec<WarmProcess> {
        let limit = self.settings.idle_limit;
        let mut idle = self.lock();
        let (expired, kept): (Vec<WarmProcess>, Vec<WarmProcess>) = idle
            .drain(..)
            .partition(|process| process.idle_for(now) >= limit);
        *idle = kept;
        expired
    }

    fn lock(&self) -> MutexGuard<'_, Vec<WarmProcess>> {
        self.idle.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn start_reaper(self: &Arc<Self>) {
        let pool = Arc::downgrade(self);
        self.reaper.get_or_init(|| Self::spawn_reaper(pool));
    }

    fn spawn_reaper(pool: Weak<Self>) {
        drop(tokio::spawn(Self::reap_while_open(pool)));
    }

    async fn reap_while_open(pool: Weak<Self>) {
        let mut open = true;
        while open {
            sleep(Self::REAP_EVERY).await;
            open = Self::reap_once(&pool).await;
        }
    }

    async fn reap_once(pool: &Weak<Self>) -> bool {
        let Some(pool) = pool.upgrade() else {
            return false;
        };
        let expired = pool.take_expired(Instant::now());
        Self::retire_all(expired).await;
        !pool.closed.load(Ordering::SeqCst)
    }

    async fn retire_all(processes: Vec<WarmProcess>) {
        for process in processes {
            process.retire().await;
        }
    }
}
