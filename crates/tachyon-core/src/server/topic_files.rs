use super::TopicHub;
use crate::handler::TopicFolder;
use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt as _};
use cap_std::fs::{Dir, File, OpenOptions};
use std::sync::{Arc, Mutex, PoisonError};

#[derive(Clone)]
pub(in crate::server) struct TopicFiles {
    pub(in crate::server) folder: TopicFolder,
    pub(in crate::server) directory: Arc<Mutex<Option<Arc<Dir>>>>,
}

impl TopicFiles {
    pub(in crate::server) fn open_directory(folder: &TopicFolder) -> Result<Option<Dir>, String> {
        folder.open().map_err(String::from)
    }

    pub(in crate::server) fn open_file(
        &self,
        topic: &str,
    ) -> Result<Option<std::fs::File>, String> {
        let topics = self.retained_directory()?;
        topics
            .map(|topics| Self::open_in(&topics, topic))
            .transpose()
            .map(Option::flatten)
    }

    fn retained_directory(&self) -> Result<Option<Arc<Dir>>, String> {
        let mut retained = self
            .directory
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if retained.is_none() {
            *retained = Self::open_directory(&self.folder)?.map(Arc::new);
        }
        Ok(retained.clone())
    }

    fn open_in(topics: &Dir, topic: &str) -> Result<Option<std::fs::File>, String> {
        let file = Self::open_existing(topics, topic)?;
        file.map(Self::checked_file).transpose()
    }

    fn open_existing(topics: &Dir, topic: &str) -> Result<Option<File>, String> {
        let mut options = OpenOptions::new();
        options.read(true).follow(FollowSymlinks::No);
        match topics.open_with(format!("{topic}.jsonl"), &options) {
            Ok(file) => Ok(Some(file)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(_) => Err(String::from("Topic stream cannot be opened.")),
        }
    }

    fn checked_file(file: File) -> Result<std::fs::File, String> {
        let metadata = file
            .metadata()
            .map_err(|_| String::from("Topic stream cannot be inspected."))?;
        Self::check_bounded(&metadata)?;
        Ok(file.into_std())
    }

    fn check_bounded(metadata: &cap_std::fs::Metadata) -> Result<(), String> {
        if !metadata.is_file() || metadata.len() > TopicHub::MAX_LOG_BYTES {
            return Err(String::from("Topic stream must be a bounded regular file."));
        }
        Ok(())
    }
}
