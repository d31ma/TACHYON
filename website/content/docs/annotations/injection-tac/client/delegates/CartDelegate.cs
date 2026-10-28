[Delegate]
public sealed class CartDelegate {
    [Needs]
    private readonly CartPricesDelegate _prices;

    public int Settle(int subtotal) {
        int surcharge = _prices.Surcharge();
        return subtotal + surcharge;
    }

    public int Count() {
        return _prices.Count();
    }
}
