[Helper]
public sealed class CartHelper {
    private readonly CartPricesHelper _prices;

    public CartHelper() {
        _prices = new CartPricesHelper();
    }

    public int Settle(int subtotal) {
        int surcharge = _prices.Surcharge();
        return subtotal + surcharge;
    }

    public int Count() {
        return _prices.Count();
    }
}
