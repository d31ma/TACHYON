[Helper]
public sealed class CartHelper {
    [Needs]
    private readonly CartPricesHelper _prices;

    public int Settle(int subtotal) {
        int surcharge = _prices.Surcharge();
        return subtotal + surcharge;
    }

    public int Count() {
        return _prices.Count();
    }
}
