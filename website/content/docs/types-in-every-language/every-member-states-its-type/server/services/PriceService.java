import models.Price;

@Service
class PriceService {
    private static final String CURRENCY = "EUR";

    Price priceFor(int amount, int count) {
        return new Price(amount * count, CURRENCY);
    }
}
