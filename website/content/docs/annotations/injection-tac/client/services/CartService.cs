[Service]
[Lifetime({"value":"prototype"})]
public sealed class CartService {
    [Needs]
    private readonly CartPricesDelegate _prices;
    [Needs]
    private readonly CartDelegate _cart;

    public int Total() {
        int subtotal = _prices.Subtotal();
        return _cart.Settle(subtotal);
    }
}
