use crate::models::Ledger;

#[RequestRoute("/ledger")]
#[Public]
struct LedgerController {
    #[Needs]
    #[Named("LedgerService")]
    service: std::sync::Arc<LedgerService>,
    #[Needs]
    #[Named("LedgerHelper")]
    settlement: std::sync::Arc<LedgerHelper>,
}
impl LedgerController {
    #[GetRoute]
    fn get_ledger(&self) -> Ledger {
        let subtotal = self.service.subtotal();
        let total = self.settlement.settle(subtotal);
        Ledger { total }
    }
}
