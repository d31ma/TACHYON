[Service]
[Lifetime({"value":"prototype"})]
public sealed class CartService {
    [Needs]
    private readonly CartPricesHelper _prices;
    [Needs]
    private readonly CartHelper _cart;

    public int Total() {
        int subtotal = _prices.Subtotal();
        return _cart.Settle(subtotal);
    }
}
