use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use tokio::sync::RwLock;

use super::collection_pass::CollectionPass;

pub(super) struct CollectionGates {
    collections: Mutex<HashMap<String, Arc<RwLock<()>>>>,
    every_collection: Arc<RwLock<()>>,
}

impl CollectionGates {
    pub(super) fn new() -> Self {
        Self {
            collections: Mutex::new(HashMap::new()),
            every_collection: Arc::new(RwLock::new(())),
        }
    }

    pub(super) async fn writer(&self, collection: &str) -> CollectionPass {
        let shared = self.gate(collection).read_owned().await;
        CollectionPass::new(vec![Box::new(shared)])
    }

    pub(super) async fn sql_writer(&self) -> CollectionPass {
        let whole = Arc::clone(&self.every_collection).write_owned().await;
        CollectionPass::new(vec![Box::new(whole)])
    }

    pub(super) async fn transaction(&self, collection: &str) -> CollectionPass {
        let every = Arc::clone(&self.every_collection).read_owned().await;
        let only = self.gate(collection).write_owned().await;
        CollectionPass::new(vec![Box::new(every), Box::new(only)])
    }

    fn gate(&self, collection: &str) -> Arc<RwLock<()>> {
        let mut collections = self
            .collections
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let gate = collections.entry(collection.to_owned()).or_default();
        Arc::clone(gate)
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::CollectionGates;

    struct Timing;

    impl Timing {
        const SHORT: Duration = Duration::from_millis(50);
        const LIFETIME: Duration = Duration::from_millis(200);
    }

    #[tokio::test]
    async fn writers_to_one_collection_do_not_wait_for_each_other() {
        let gates = CollectionGates::new();
        let _first = gates.writer("orders").await;
        let second = tokio::time::timeout(Timing::SHORT, gates.writer("orders")).await;
        assert!(second.is_ok());
    }

    #[tokio::test]
    async fn a_writer_waits_while_a_transaction_holds_its_collection() {
        let gates = CollectionGates::new();
        let transaction = gates.transaction("orders").await;
        let waited = tokio::time::timeout(Timing::SHORT, gates.writer("orders")).await;
        assert!(waited.is_err());
        let other = tokio::time::timeout(Timing::SHORT, gates.writer("people")).await;
        assert!(other.is_ok());
        drop(transaction);
        let after = tokio::time::timeout(Timing::SHORT, gates.writer("orders")).await;
        assert!(after.is_ok());
    }

    #[tokio::test]
    async fn an_sql_write_waits_while_any_transaction_is_open() {
        let gates = CollectionGates::new();
        let transaction = gates.transaction("orders").await;
        let waited = tokio::time::timeout(Timing::SHORT, gates.sql_writer()).await;
        assert!(waited.is_err());
        drop(transaction);
        let after = tokio::time::timeout(Timing::SHORT, gates.sql_writer()).await;
        assert!(after.is_ok());
    }

    #[tokio::test]
    async fn a_held_pass_is_given_back_at_its_expiry() {
        let gates = CollectionGates::new();
        let pass = gates.transaction("orders").await;
        let _release = pass.hold_until_released_or(Instant::now() + Timing::LIFETIME);
        let waited = tokio::time::timeout(Timing::SHORT, gates.writer("orders")).await;
        assert!(waited.is_err());
        let after = tokio::time::timeout(Timing::LIFETIME * 2, gates.writer("orders")).await;
        assert!(after.is_ok());
    }

    #[tokio::test]
    async fn a_held_pass_is_given_back_when_its_release_is_dropped() {
        let gates = CollectionGates::new();
        let pass = gates.transaction("orders").await;
        let release = pass.hold_until_released_or(Instant::now() + Duration::from_secs(60));
        drop(release);
        let after = tokio::time::timeout(Timing::LIFETIME, gates.writer("orders")).await;
        assert!(after.is_ok());
    }
}
