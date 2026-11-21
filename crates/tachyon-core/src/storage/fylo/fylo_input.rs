use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use tokio::io::AsyncWriteExt;
use tokio::process::ChildStdin;

use super::FyloError;

pub(super) struct FyloInput {
    stdin: Mutex<Option<ChildStdin>>,
    closed: AtomicBool,
}

impl FyloInput {
    pub(super) fn new(stdin: ChildStdin) -> Self {
        Self {
            stdin: Mutex::new(Some(stdin)),
            closed: AtomicBool::new(false),
        }
    }

    pub(super) async fn write_line(&self, line: &[u8]) -> Result<(), FyloError> {
        let mut stdin = self.slot().take().ok_or_else(Self::refused)?;
        stdin.write_all(line).await.map_err(|_| Self::refused())?;
        self.put_back(stdin);
        Ok(())
    }

    pub(super) fn close(&self) {
        self.closed.store(true, Ordering::Release);
        self.slot().take();
    }

    fn put_back(&self, stdin: ChildStdin) {
        let mut slot = self.slot();
        if !self.closed.load(Ordering::Acquire) {
            *slot = Some(stdin);
        }
    }

    fn refused() -> FyloError {
        FyloError::protocol("FYLO subprocess is not accepting requests.")
    }

    fn slot(&self) -> MutexGuard<'_, Option<ChildStdin>> {
        self.stdin.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
