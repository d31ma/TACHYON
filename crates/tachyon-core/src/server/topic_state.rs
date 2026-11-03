use super::{TopicFailure, TopicHub, TopicReplay};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

pub(in crate::server) type TopicRecord = (u64, String, bool);

pub(in crate::server) struct TopicState {
    pub(in crate::server) sender: broadcast::Sender<TopicRecord>,
    pub(in crate::server) replay: Mutex<VecDeque<TopicRecord>>,
    pub(in crate::server) replay_evicted: AtomicBool,
    pub(in crate::server) subscribers: AtomicUsize,
    pub(in crate::server) started: AtomicBool,
    pub(in crate::server) cancellation: CancellationToken,
}

impl TopicState {
    pub(in crate::server) fn publish_record(&self, position: u64, frame: String) {
        self.retain_frame(position, frame.clone(), false);
        let _ = self.sender.send((position, frame, false));
    }

    pub(in crate::server) fn publish_error(&self, position: u64, failure: TopicFailure) {
        let frame = failure.frame();
        self.retain_frame(position, frame.clone(), true);
        let _ = self.sender.send((position, frame, true));
    }

    pub(in crate::server) fn retain_frame(&self, position: u64, frame: String, terminal: bool) {
        let mut replay = self.replay.lock().unwrap_or_else(PoisonError::into_inner);
        self.evict_oldest_when_full(&mut replay);
        replay.push_back((position, frame, terminal));
    }

    pub(in crate::server) fn replay_snapshot(&self, requested: Option<u64>) -> TopicReplay {
        let replay = self.replay.lock().unwrap_or_else(PoisonError::into_inner);
        let oldest = replay.front().map(|record| record.0);
        let resume = requested.unwrap_or(oldest.unwrap_or(0));
        let cursor_gap = self.cursor_gap(requested, oldest, resume);
        let frames = replay
            .iter()
            .filter(|record| Self::replays(record, resume))
            .cloned()
            .collect();
        TopicReplay {
            resume,
            frames,
            cursor_gap,
        }
    }

    fn evict_oldest_when_full(&self, replay: &mut MutexGuard<'_, VecDeque<TopicRecord>>) {
        if replay.len() == TopicHub::REPLAY_RECORDS {
            replay.pop_front();
            self.replay_evicted.store(true, Ordering::Release);
        }
    }

    fn cursor_gap(&self, requested: Option<u64>, oldest: Option<u64>, resume: u64) -> bool {
        requested.is_some()
            && self.replay_evicted.load(Ordering::Acquire)
            && oldest.is_some_and(|oldest| resume < oldest)
    }

    fn replays(record: &TopicRecord, resume: u64) -> bool {
        record.2 || record.0 >= resume
    }
}
