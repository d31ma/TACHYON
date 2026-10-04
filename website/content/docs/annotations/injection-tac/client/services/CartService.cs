[Service]
[Lifetime({"value":"prototype"})]
public sealed class CartService {
    private readonly CartPricesHelper _prices;
    private readonly CartHelper _cart;

    public CartService() {
        _prices = new CartPricesHelper();
        _cart = new CartHelper();
    }

    public int Total() {
        int subtotal = _prices.Subtotal();
        return _cart.Settle(subtotal);
    }
}
