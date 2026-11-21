use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Instant;

use super::collection_gates::CollectionGates;
use super::collection_pass::CollectionPass;
use super::open_fylo_transaction::OpenFyloTransaction;
use crate::Failure;
use crate::storage::{ManagedStorage, failure};

pub(super) struct FyloTransactions {
    open: Mutex<BTreeMap<String, OpenFyloTransaction>>,
    gates: CollectionGates,
}

impl FyloTransactions {
    const UNKNOWN: &'static str = "Unknown, expired or already settled transaction; start a new one.";

    pub(super) fn new() -> Self {
        Self {
            open: Mutex::new(BTreeMap::new()),
            gates: CollectionGates::new(),
        }
    }

    pub(super) fn gates(&self) -> &CollectionGates {
        &self.gates
    }

    pub(super) fn begin(&self) -> Result<String, Failure> {
        let mut open = self.live_transactions();
        Self::refuse_when_full(&open)?;
        let id = crate::ttid::generate();
        let expires = Instant::now() + ManagedStorage::TRANSACTION_TIMEOUT;
        open.insert(id.clone(), OpenFyloTransaction::new(expires));
        Ok(id)
    }

    pub(super) fn binding(&self, id: &str) -> Result<Option<(String, String)>, Failure> {
        let open = self.live_transactions();
        let transaction = open.get(id).ok_or_else(|| failure(Self::UNKNOWN))?;
        Ok(transaction.binding())
    }

    pub(super) fn expires(&self, id: &str) -> Result<Instant, Failure> {
        let open = self.live_transactions();
        let transaction = open.get(id).ok_or_else(|| failure(Self::UNKNOWN))?;
        Ok(transaction.expires())
    }

    pub(super) fn bind(
        &self,
        id: &str,
        collection: &str,
        handle: String,
        pass: CollectionPass,
    ) -> Result<(), Failure> {
        let mut open = self.live_transactions();
        let transaction = open.get_mut(id).ok_or_else(|| failure(Self::UNKNOWN))?;
        let release = pass.hold_until_released_or(transaction.expires());
        transaction.bind(collection, handle, release);
        Ok(())
    }

    pub(super) fn settle(&self, id: &str) -> Result<Option<String>, Failure> {
        let mut open = self.live_transactions();
        let transaction = open.remove(id).ok_or_else(|| failure(Self::UNKNOWN))?;
        Ok(transaction.handle())
    }

    pub(super) fn forget(&self, id: &str) {
        self.live_transactions().remove(id);
    }

    pub(super) fn close(&self) {
        self.live_transactions().clear();
    }

    fn live_transactions(&self) -> MutexGuard<'_, BTreeMap<String, OpenFyloTransaction>> {
        let mut open = self.open.lock().unwrap_or_else(PoisonError::into_inner);
        let now = Instant::now();
        open.retain(|_, transaction| transaction.live(now));
        open
    }

    fn refuse_when_full(open: &BTreeMap<String, OpenFyloTransaction>) -> Result<(), Failure> {
        if open.len() >= ManagedStorage::MAX_TRANSACTIONS {
            return Err(failure(
                "Open transaction limit reached; commit or roll back before starting another.",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::FyloTransactions;
    use crate::storage::fylo::collection_pass::CollectionPass;

    #[test]
    fn a_new_transaction_has_no_collection_until_its_first_write() {
        let transactions = FyloTransactions::new();
        let id = transactions.begin().unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(transactions.binding(&id).ok(), Some(None));
    }

    #[tokio::test]
    async fn a_bound_transaction_names_its_collection_and_fylo_handle() {
        let transactions = FyloTransactions::new();
        let id = transactions.begin().unwrap_or_else(|error| panic!("{error}"));
        let bound = transactions.bind(&id, "orders", String::from("t1"), CollectionPass::none());
        assert!(bound.is_ok());
        let binding = Some(Some((String::from("orders"), String::from("t1"))));
        assert_eq!(transactions.binding(&id).ok(), binding);
        assert_eq!(transactions.settle(&id).ok(), Some(Some(String::from("t1"))));
    }

    #[test]
    fn a_settled_transaction_is_unknown() {
        let transactions = FyloTransactions::new();
        let id = transactions.begin().unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(transactions.settle(&id).ok(), Some(None));
        assert!(transactions.settle(&id).is_err());
        assert!(transactions.binding(&id).is_err());
    }

    #[test]
    fn at_most_four_transactions_are_open_at_once() {
        let transactions = FyloTransactions::new();
        let opened = (0..4).filter_map(|_| transactions.begin().ok()).count();
        assert_eq!(opened, 4);
        assert!(transactions.begin().is_err());
    }
}
