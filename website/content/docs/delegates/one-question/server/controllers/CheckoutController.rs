use crate::models::Receipt;

#[RequestRoute("/checkout")]
#[Public]
struct CheckoutController {
    #[Needs]
    service: std::sync::Arc<CheckoutService>,
}
impl CheckoutController {
    #[GetRoute]
    fn get_checkout(&self) -> Receipt {
        let total = self.service.total();
        Receipt { total }
    }
}
