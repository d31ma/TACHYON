#[Service]
#[Lifetime({"value":"prototype"})]
struct CartService {
    #[Needs]
    cart_prices_delegate: CartPricesDelegate,
    #[Needs]
    cart_delegate: CartDelegate,
}
impl CartService {
    pub fn total(&self) -> i64 {
        let subtotal = self.cart_prices_delegate.subtotal();
        self.cart_delegate.settle(subtotal)
    }
}
