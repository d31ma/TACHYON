@RequestRoute("/price")
@Public
class PriceController {
  /** @type {PriceService} */
  @Needs
  priceService

  /** @returns {Price} */
  @GetRoute
  getPrice() {
    return this.priceService.priceFor(20, 3)
  }
}
