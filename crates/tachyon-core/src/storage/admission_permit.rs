use std::future::Future;
use std::sync::Arc;

use tokio::sync::OwnedSemaphorePermit;
use tokio::task::LocalKey;

pub(super) struct AdmissionPermit;

impl AdmissionPermit {
    fn held() -> &'static LocalKey<Arc<OwnedSemaphorePermit>> {
        tokio::task_local! {
            static HELD: Arc<OwnedSemaphorePermit>;
        }
        &HELD
    }

    pub(super) async fn hold_during<Work: Future>(
        permit: OwnedSemaphorePermit,
        work: Work,
    ) -> Work::Output {
        Self::held().scope(Arc::new(permit), work).await
    }

    #[cfg(feature = "storage-fylo-in-process")]
    pub(super) fn shared() -> Option<Arc<OwnedSemaphorePermit>> {
        Self::held().try_with(Arc::clone).ok()
    }
}
