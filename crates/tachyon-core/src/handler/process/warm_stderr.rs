use super::Drained;
use std::sync::{Arc, Mutex, PoisonError};
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::ChildStderr;
use tokio::task::JoinHandle;
use tokio::time::{Instant, timeout_at};

pub(super) struct WarmStderr {
    window: Arc<Mutex<Drained>>,
    task: JoinHandle<()>,
}

impl WarmStderr {
    const CHUNK_BYTES: usize = 8 * 1024;

    pub(super) fn spawn(stderr: ChildStderr, limit: usize) -> Self {
        let window = Arc::new(Mutex::new(Self::empty()));
        let task = tokio::spawn(Self::read_all(stderr, Arc::clone(&window), limit));
        Self { window, task }
    }

    pub(super) fn take(&self) -> Drained {
        let mut window = self.window.lock().unwrap_or_else(PoisonError::into_inner);
        std::mem::replace(&mut *window, Self::empty())
    }

    pub(super) async fn take_after_exit(mut self, deadline: Instant) -> Drained {
        let _finished = timeout_at(deadline, &mut self.task).await;
        self.task.abort();
        self.take()
    }

    fn empty() -> Drained {
        Drained {
            bytes: Vec::new(),
            overflow: false,
            read_error: false,
        }
    }

    async fn read_all<R>(mut stderr: R, window: Arc<Mutex<Drained>>, limit: usize)
    where
        R: AsyncRead + Unpin,
    {
        let mut chunk = vec![0_u8; Self::CHUNK_BYTES];
        let mut open = true;
        while open {
            open = Self::read_chunk(&mut stderr, &mut chunk, &window, limit).await;
        }
    }

    async fn read_chunk<R>(
        stderr: &mut R,
        chunk: &mut [u8],
        window: &Mutex<Drained>,
        limit: usize,
    ) -> bool
    where
        R: AsyncRead + Unpin,
    {
        let read = stderr.read(chunk).await.unwrap_or(0);
        let mut kept = window.lock().unwrap_or_else(PoisonError::into_inner);
        kept.keep(&chunk[..read], limit);
        read > 0
    }
}
