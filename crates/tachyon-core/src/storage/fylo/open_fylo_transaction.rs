use std::time::Instant;

use tokio::sync::oneshot;

pub(super) struct OpenFyloTransaction {
    expires: Instant,
    collection: Option<String>,
    handle: Option<String>,
    release: Option<oneshot::Sender<()>>,
}

impl OpenFyloTransaction {
    pub(super) fn new(expires: Instant) -> Self {
        Self {
            expires,
            collection: None,
            handle: None,
            release: None,
        }
    }

    pub(super) fn expires(&self) -> Instant {
        self.expires
    }

    pub(super) fn live(&self, now: Instant) -> bool {
        self.expires > now
    }

    pub(super) fn binding(&self) -> Option<(String, String)> {
        self.collection.clone().zip(self.handle.clone())
    }

    pub(super) fn handle(&self) -> Option<String> {
        self.handle.clone()
    }

    pub(super) fn bind(&mut self, collection: &str, handle: String, release: oneshot::Sender<()>) {
        self.collection = Some(collection.to_owned());
        self.handle = Some(handle);
        self.release = Some(release);
    }
}
