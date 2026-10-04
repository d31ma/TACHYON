#[Helper]
struct CartHelper {
    #[Needs]
    cart_prices_helper: CartPricesHelper,
}
impl CartHelper {
    fn settle(&self, subtotal: i64) -> i64 {
        let surcharge = self.cart_prices_helper.surcharge();
        subtotal + surcharge
    }

    fn count(&self) -> i64 {
        self.cart_prices_helper.count()
    }
}
