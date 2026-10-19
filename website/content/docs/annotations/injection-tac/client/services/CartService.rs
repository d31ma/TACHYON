#[Service]
#[Lifetime({"value":"prototype"})]
struct CartService {
    #[Needs]
    cart_prices_helper: CartPricesHelper,
    #[Needs]
    cart_helper: CartHelper,
}
impl CartService {
    pub fn total(&self) -> i64 {
        let subtotal = self.cart_prices_helper.subtotal();
        self.cart_helper.settle(subtotal)
    }
}
