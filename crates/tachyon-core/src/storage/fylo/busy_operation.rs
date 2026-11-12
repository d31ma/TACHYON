use std::sync::{Mutex, PoisonError};

pub(super) struct BusyOperation {
    name: Mutex<Option<String>>,
}

impl BusyOperation {
    pub(super) fn new() -> Self {
        Self {
            name: Mutex::new(None),
        }
    }

    pub(super) fn mark(&self, operation: &str) {
        *self.slot() = Some(operation.to_owned());
    }

    pub(super) fn clear(&self) {
        *self.slot() = None;
    }

    pub(super) fn name(&self) -> Option<String> {
        self.slot().clone()
    }

    fn slot(&self) -> std::sync::MutexGuard<'_, Option<String>> {
        self.name.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::BusyOperation;

    #[test]
    fn a_marked_operation_stays_until_it_is_cleared() {
        let busy = BusyOperation::new();
        assert_eq!(busy.name(), None);
        busy.mark("putData");
        assert_eq!(busy.name().as_deref(), Some("putData"));
        busy.clear();
        assert_eq!(busy.name(), None);
    }
}
