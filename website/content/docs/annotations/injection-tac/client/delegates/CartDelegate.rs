#[Delegate]
struct CartDelegate {
    #[Needs]
    cart_prices_delegate: CartPricesDelegate,
}
impl CartDelegate {
    fn settle(&self, subtotal: i64) -> i64 {
        let surcharge = self.cart_prices_delegate.surcharge();
        subtotal + surcharge
    }

    fn count(&self) -> i64 {
        self.cart_prices_delegate.count()
    }
}
