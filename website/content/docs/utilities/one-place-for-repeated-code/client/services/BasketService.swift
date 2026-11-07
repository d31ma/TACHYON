@Service
final class BasketService {
    private let total: Int = 1250

    func totalLabel() -> String {
        PriceUtils.label(total)
    }
}
