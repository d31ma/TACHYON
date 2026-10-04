@Service
class BasketService {
    private final int total = 1250;

    String totalLabel() {
        return PriceUtils.label(total);
    }
}
