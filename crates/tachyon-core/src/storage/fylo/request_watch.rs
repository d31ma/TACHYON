use serde_json::Value;
use tokio::process::Child;

use super::FyloError;
use super::interruption::Interruption;

pub(super) struct RequestWatch<'process> {
    child: &'process mut Child,
    interruption: &'process mut Option<Interruption>,
    process_usable: bool,
}

impl<'process> RequestWatch<'process> {
    pub(super) fn start(
        child: &'process mut Child,
        interruption: &'process mut Option<Interruption>,
        operation: &str,
    ) -> Self {
        *interruption = Some(Interruption::new(operation));
        Self {
            child,
            interruption,
            process_usable: false,
        }
    }

    pub(super) fn finish(mut self, answer: &Result<Value, FyloError>) {
        self.process_usable = !answer.as_ref().is_err_and(FyloError::broke_process);
    }

    fn settle(&mut self) {
        if self.process_usable {
            *self.interruption = None;
            return;
        }
        self.stop_process();
    }

    fn stop_process(&mut self) {
        self.interruption
            .iter_mut()
            .for_each(Interruption::mark_killed);
        let _already_exited = self.child.start_kill();
    }
}

impl Drop for RequestWatch<'_> {
    fn drop(&mut self) {
        self.settle();
    }
}
