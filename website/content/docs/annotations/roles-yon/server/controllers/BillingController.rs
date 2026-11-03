use crate::models::Bill;

#[RequestRoute("/billing")]
#[Public]
struct BillingController {
    #[Needs]
    service: std::sync::Arc<BillingService>,
}
impl BillingController {
    #[GetRoute]
    fn get_billing(&self) -> Bill {
        let total = self.service.total();
        Bill { total }
    }
}
