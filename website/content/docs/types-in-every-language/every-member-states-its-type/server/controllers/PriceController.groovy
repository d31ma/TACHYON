import models.Price

@RequestRoute("/price")
@Public
class PriceController {
    @Needs
    PriceService priceService

    @GetRoute
    Price getPrice() {
        return priceService.priceFor(20, 3)
    }
}
