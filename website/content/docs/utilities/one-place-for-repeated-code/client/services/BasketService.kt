@Service
class BasketService {
    private val total: Int = 1250

    fun totalLabel(): String = PriceUtils.label(total)
}
