import models.Price

@Service
class PriceService {
    private val currency: String = "EUR"

    fun priceFor(amount: Int, count: Int): Price = Price(amount * count, currency)
}
