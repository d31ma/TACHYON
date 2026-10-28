use crate::models::Ledger;

#[RequestRoute("/ledger")]
#[Public]
struct LedgerController {
    #[Needs]
    #[Named("LedgerService")]
    service: std::sync::Arc<LedgerService>,
    #[Needs]
    #[Named("LedgerDelegate")]
    settlement: std::sync::Arc<LedgerDelegate>,
}
impl LedgerController {
    #[GetRoute]
    fn get_ledger(&self) -> Ledger {
        let subtotal = self.service.subtotal();
        let total = self.settlement.settle(subtotal);
        Ledger { total }
    }
}
