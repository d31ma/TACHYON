[Service]
public sealed class BasketService {
    private readonly int total = 1250;

    public string TotalLabel() {
        return PriceUtils.Label(total);
    }
}
