using Models;

[Service]
class PriceService {
    private const string Currency = "EUR";

    public Price PriceFor(int amount, int count) {
        return new Price(amount * count, Currency);
    }
}
