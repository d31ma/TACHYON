use super::{ProducerTasks, ServerRuntime, TopicAdmission, TopicFiles, TopicState, TopicTailer};
use crate::Failure;
use crate::failure::diagnostic;
use crate::handler::TopicFolder;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub(in crate::server) struct TopicHub {
    pub(in crate::server) files: TopicFiles,
    pub(in crate::server) topics: Arc<Mutex<HashMap<String, Arc<TopicState>>>>,
    pub(in crate::server) subscribers: Arc<AtomicUsize>,
    cancellation: CancellationToken,
    producers: ProducerTasks,
}

impl TopicHub {
    pub(in crate::server) const ENDPOINT: &str = "/.tachyon/topics/";
    #[cfg(test)]
    pub(in crate::server) const DIRECTORY: &str = "data/topics";
    pub(in crate::server) const MAX_NAME_BYTES: usize = 64;
    pub(in crate::server) const MAX_LOG_BYTES: u64 = 16 * 1024 * 1024;
    pub(in crate::server) const MAX_RECORD_BYTES: usize = 64 * 1024;
    pub(in crate::server) const POLL_INTERVAL: Duration = Duration::from_millis(50);
    const MAX_SUBSCRIBERS: usize = 128;
    pub(in crate::server) const MAX_SUBSCRIBERS_PER_TOPIC: usize = 32;
    pub(in crate::server) const MAX_ACTIVE_TOPICS: usize = 32;
    pub(in crate::server) const REPLAY_RECORDS: usize = 256;
    pub(in crate::server) const FIRST_POSITION: u64 = 0;
    pub(in crate::server) const BROADCAST_RECORDS: usize = 128;
    pub(in crate::server) const STREAM_BUFFER_BYTES: usize = 64 * 1024;

    pub(in crate::server) fn new(
        folder: TopicFolder,
        cancellation: CancellationToken,
        producers: ProducerTasks,
    ) -> Result<Self, String> {
        let directory = TopicFiles::open_directory(&folder)?.map(Arc::new);
        Ok(Self {
            files: TopicFiles {
                folder,
                directory: Arc::new(Mutex::new(directory)),
            },
            topics: Arc::new(Mutex::new(HashMap::new())),
            subscribers: Arc::new(AtomicUsize::new(0)),
            cancellation,
            producers,
        })
    }

    pub(in crate::server) fn open(
        folder: TopicFolder,
        runtime: &ServerRuntime,
    ) -> Result<Self, Failure> {
        Self::new(
            folder,
            runtime.cancellation.clone(),
            runtime.producers.clone(),
        )
        .map_err(|message| {
            Failure::one(diagnostic(
                1303,
                message,
                Some(String::from(
                    "Keep the topic log folder (topics.directory, data/topics unless set) a regular folder that no symbolic link leads out of the project.",
                )),
                None,
            ))
        })
    }

    pub(in crate::server) fn lock_topics(
        &self,
    ) -> MutexGuard<'_, HashMap<String, Arc<TopicState>>> {
        self.topics.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(in crate::server) fn subscribe(
        &self,
        topic: &str,
    ) -> Result<(Arc<TopicState>, TopicAdmission), &'static str> {
        self.reserve_subscriber()?;
        let (state, created) = self.join_or_create_topic(topic)?;
        self.start_when_created(topic, &state, created)?;
        Ok((Arc::clone(&state), self.admission(topic, state)))
    }

    fn reserve_subscriber(&self) -> Result<(), &'static str> {
        if self.subscribers.fetch_add(1, Ordering::AcqRel) >= Self::MAX_SUBSCRIBERS {
            self.subscribers.fetch_sub(1, Ordering::AcqRel);
            return Err("Topic subscriber capacity is exhausted.");
        }
        Ok(())
    }

    fn join_or_create_topic(&self, topic: &str) -> Result<(Arc<TopicState>, bool), &'static str> {
        let mut topics = self.lock_topics();
        let existing = topics.get(topic).cloned();
        self.join_or_create(&mut topics, topic, existing)
    }

    fn join_or_create(
        &self,
        topics: &mut HashMap<String, Arc<TopicState>>,
        topic: &str,
        existing: Option<Arc<TopicState>>,
    ) -> Result<(Arc<TopicState>, bool), &'static str> {
        match existing {
            Some(state) => self.join_existing(state),
            None => self.create_topic(topics, topic),
        }
    }

    fn join_existing(
        &self,
        state: Arc<TopicState>,
    ) -> Result<(Arc<TopicState>, bool), &'static str> {
        self.check_topic_room(&state)?;
        Ok((state, false))
    }

    fn check_topic_room(&self, state: &TopicState) -> Result<(), &'static str> {
        if state.subscribers.fetch_add(1, Ordering::AcqRel) >= Self::MAX_SUBSCRIBERS_PER_TOPIC {
            state.subscribers.fetch_sub(1, Ordering::AcqRel);
            self.subscribers.fetch_sub(1, Ordering::AcqRel);
            return Err("Topic subscriber capacity is exhausted.");
        }
        Ok(())
    }

    fn create_topic(
        &self,
        topics: &mut HashMap<String, Arc<TopicState>>,
        topic: &str,
    ) -> Result<(Arc<TopicState>, bool), &'static str> {
        self.check_topic_count(topics.len())?;
        let state = Arc::new(self.new_topic_state());
        topics.insert(topic.to_owned(), Arc::clone(&state));
        Ok((state, true))
    }

    fn check_topic_count(&self, active: usize) -> Result<(), &'static str> {
        if active >= Self::MAX_ACTIVE_TOPICS {
            self.subscribers.fetch_sub(1, Ordering::AcqRel);
            return Err("Topic capacity is exhausted.");
        }
        Ok(())
    }

    fn new_topic_state(&self) -> TopicState {
        let (sender, _) = broadcast::channel(Self::BROADCAST_RECORDS);
        TopicState {
            sender,
            replay: Mutex::new(VecDeque::with_capacity(Self::REPLAY_RECORDS)),
            replay_evicted: AtomicBool::new(false),
            subscribers: AtomicUsize::new(1),
            started: AtomicBool::new(false),
            cancellation: self.cancellation.child_token(),
        }
    }

    fn start_when_created(
        &self,
        topic: &str,
        state: &Arc<TopicState>,
        created: bool,
    ) -> Result<(), &'static str> {
        if created && !state.started.swap(true, Ordering::AcqRel) {
            return self.start_tailer(topic, state);
        }
        Ok(())
    }

    fn start_tailer(&self, topic: &str, state: &Arc<TopicState>) -> Result<(), &'static str> {
        let tailer = TopicTailer::new(
            self.files.clone(),
            topic.to_owned(),
            Arc::clone(state),
            state.cancellation.clone(),
        );
        let accepted = self.producers.spawn(tailer.run());
        self.tailer_result(accepted, topic, state)
    }

    fn tailer_result(
        &self,
        accepted: bool,
        topic: &str,
        state: &Arc<TopicState>,
    ) -> Result<(), &'static str> {
        if accepted {
            return Ok(());
        }
        state.started.store(false, Ordering::Release);
        drop(self.admission(topic, Arc::clone(state)));
        Err("Topic stream is unavailable.")
    }

    fn admission(&self, topic: &str, state: Arc<TopicState>) -> TopicAdmission {
        TopicAdmission {
            hub: self.clone(),
            name: topic.to_owned(),
            topic: state,
        }
    }
}
