import models.Price

@RequestRoute("/price")
@Public
class PriceController {
    @Needs
    val priceService: PriceService

    @GetRoute
    fun getPrice(): Price = priceService.priceFor(20, 3)
}
