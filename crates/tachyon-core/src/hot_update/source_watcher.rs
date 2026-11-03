use super::SourceChanges;
use crate::Failure;
use crate::failure::diagnostic;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher as _};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc;

pub(crate) struct SourceWatcher {
    _watcher: RecommendedWatcher,
    receiver: mpsc::Receiver<Result<Event, notify::Error>>,
    overflowed: Arc<AtomicBool>,
}

impl SourceWatcher {
    const QUEUE_CAPACITY: usize = 1_024;
    const SOURCE_FOLDERS: [&'static str; 2] = ["client", "server"];

    pub(crate) fn start(project_root: &Path) -> Result<Self, Failure> {
        let (sender, receiver) = mpsc::channel(Self::QUEUE_CAPACITY);
        let overflowed = Arc::new(AtomicBool::new(false));
        let callback_overflowed = Arc::clone(&overflowed);
        let mut watcher = notify::recommended_watcher(move |event| {
            Self::forward(&sender, &callback_overflowed, event);
        })
        .map_err(|error| Self::watcher_failure(&error))?;
        Self::watch_sources(&mut watcher, project_root)?;
        Ok(Self {
            _watcher: watcher,
            receiver,
            overflowed,
        })
    }

    pub(crate) async fn receive(&mut self) -> Option<Result<Event, notify::Error>> {
        self.receiver.recv().await
    }

    pub(crate) fn drain(&mut self, changes: &mut SourceChanges, project_root: &Path) {
        self.drain_queue(changes, project_root);
        self.note_overflow(changes);
    }

    fn forward(
        sender: &mpsc::Sender<Result<Event, notify::Error>>,
        overflowed: &AtomicBool,
        event: Result<Event, notify::Error>,
    ) {
        if sender.try_send(event).is_err() {
            overflowed.store(true, Ordering::Release);
        }
    }

    fn watch_sources(watcher: &mut RecommendedWatcher, project_root: &Path) -> Result<(), Failure> {
        watcher
            .watch(project_root, RecursiveMode::NonRecursive)
            .map_err(|error| Self::watcher_failure(&error))?;
        Self::SOURCE_FOLDERS
            .iter()
            .map(|folder| project_root.join(folder))
            .filter(|path| path.is_dir())
            .try_for_each(|path| {
                watcher
                    .watch(&path, RecursiveMode::Recursive)
                    .map_err(|error| Self::watcher_failure(&error))
            })
    }

    fn drain_queue(&mut self, changes: &mut SourceChanges, project_root: &Path) {
        while let Ok(event) = self.receiver.try_recv() {
            Self::record_received(changes, project_root, event);
        }
    }

    fn record_received(
        changes: &mut SourceChanges,
        project_root: &Path,
        event: Result<Event, notify::Error>,
    ) {
        match event {
            Ok(event) => changes.record_event(project_root, event),
            Err(_) => changes.force_reload(),
        }
    }

    fn note_overflow(&self, changes: &mut SourceChanges) {
        if self.overflowed.swap(false, Ordering::AcqRel) {
            changes.force_reload();
        }
    }

    fn watcher_failure(error: &notify::Error) -> Failure {
        Failure::one(diagnostic(
            1305,
            format!("Cannot watch project sources: {error}"),
            Some(String::from(
                "Check source-directory permissions or run the development server with --no-watch.",
            )),
            None,
        ))
    }
}
