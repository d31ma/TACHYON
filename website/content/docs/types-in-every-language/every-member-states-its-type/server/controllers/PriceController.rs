use crate::models::Price;

#[RequestRoute("/price")]
#[Public]
struct PriceController {
    #[Needs]
    price_service: std::sync::Arc<PriceService>,
}
impl PriceController {
    #[GetRoute]
    fn get_price(&self) -> Price {
        self.price_service.price_for(20, 3)
    }
}
