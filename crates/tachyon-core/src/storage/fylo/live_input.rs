use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use super::fylo_input::FyloInput;

pub(super) struct LiveInput {
    current: Mutex<Option<Arc<FyloInput>>>,
}

impl LiveInput {
    pub(super) fn new() -> Self {
        Self {
            current: Mutex::new(None),
        }
    }

    pub(super) fn follow(&self, input: Arc<FyloInput>) {
        *self.slot() = Some(input);
    }

    pub(super) fn close(&self) {
        self.slot().iter().for_each(|input| input.close());
    }

    fn slot(&self) -> MutexGuard<'_, Option<Arc<FyloInput>>> {
        self.current.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
