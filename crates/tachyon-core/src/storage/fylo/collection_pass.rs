use std::time::Instant;

use tokio::sync::oneshot;

pub(super) struct CollectionPass {
    guards: Vec<Box<dyn Send>>,
}

impl CollectionPass {
    pub(super) fn new(guards: Vec<Box<dyn Send>>) -> Self {
        Self { guards }
    }

    pub(super) fn none() -> Self {
        Self { guards: Vec::new() }
    }

    pub(super) fn hold_until_released_or(self, expires: Instant) -> oneshot::Sender<()> {
        let (release, released) = oneshot::channel::<()>();
        tokio::spawn(Self::held(self.guards, released, expires));
        release
    }

    async fn held(guards: Vec<Box<dyn Send>>, released: oneshot::Receiver<()>, expires: Instant) {
        tokio::select! {
            _settled = released => {}
            () = tokio::time::sleep_until(expires.into()) => {}
        }
        drop(guards);
    }
}
