use crate::handler::HandlerSource;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ScheduledWorker {
    pub(super) relative: String,
    pub(super) every_seconds: u64,
    pub(super) initial_seconds: u64,
    pub(super) source: HandlerSource,
}

impl ScheduledWorker {
    pub(crate) fn relative(&self) -> &str {
        &self.relative
    }

    pub(crate) const fn initial_seconds(&self) -> u64 {
        self.initial_seconds
    }

    pub(crate) const fn every_seconds(&self) -> u64 {
        self.every_seconds
    }

    pub(crate) const fn source(&self) -> &HandlerSource {
        &self.source
    }
}
