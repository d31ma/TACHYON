mod topic_deliveries;
mod topic_delivery;
mod topic_folder;
mod topic_listeners;
mod topic_log_move;
mod topic_outbox;

pub use topic_deliveries::TopicDeliveries;
pub(crate) use topic_delivery::TopicDelivery;
pub(crate) use topic_folder::TopicFolder;
pub(crate) use topic_listeners::TopicListeners;
pub(crate) use topic_log_move::TopicLogMove;
pub(crate) use topic_outbox::TopicOutbox;

use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt as _};
use cap_std::fs::OpenOptions;
use std::io::Write as _;
use std::sync::{Arc, Mutex, PoisonError};
use tokio::sync::mpsc;

pub(crate) struct TopicBus {
    folder: TopicFolder,
    writing: Mutex<()>,
    deliveries: mpsc::Sender<TopicDelivery>,
}

impl std::fmt::Debug for TopicBus {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("TopicBus")
    }
}

impl TopicBus {
    pub(crate) const OPERATION: &'static str = "event.deliver";
    const MAX_RECORD_BYTES: usize = 64 * 1024;
    const MAX_LOG_BYTES: u64 = 16 * 1024 * 1024;
    const MAX_HOPS: u64 = 8;
    const QUEUE: usize = 128;

    pub(crate) fn open(folder: TopicFolder) -> (Arc<Self>, mpsc::Receiver<TopicDelivery>) {
        let (deliveries, receiver) = mpsc::channel(Self::QUEUE);
        let bus = Self {
            folder,
            writing: Mutex::new(()),
            deliveries,
        };
        (Arc::new(bus), receiver)
    }

    fn append(&self, slug: &str, record: &str) -> Result<(), &'static str> {
        let _writing = self.writing.lock().unwrap_or_else(PoisonError::into_inner);
        let topics = self.folder.create()?;
        let mut options = OpenOptions::new();
        options.append(true).create(true).follow(FollowSymlinks::No);
        let mut file = topics
            .open_with(format!("{slug}.jsonl"), &options)
            .map_err(|_| "Topic log cannot be opened.")?;
        let metadata = file
            .metadata()
            .map_err(|_| "Topic log cannot be inspected.")?;
        Self::check_log_room(&metadata, record)?;
        file.write_all(format!("{record}\n").as_bytes())
            .map_err(|_| "Topic log cannot be written.")
    }

    fn check_log_room(metadata: &cap_std::fs::Metadata, record: &str) -> Result<(), &'static str> {
        let size = metadata.len().saturating_add(record.len() as u64 + 1);
        if !metadata.is_file() || size > Self::MAX_LOG_BYTES {
            return Err("Topic log is full or is not a regular file.");
        }
        Ok(())
    }

    fn queue(&self, delivery: TopicDelivery) {
        let topic = delivery.topic.clone();
        if self.deliveries.try_send(delivery).is_err() {
            eprintln!("Topic delivery queue is full; server listeners skip one '{topic}' event.");
        }
    }
}
