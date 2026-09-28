import type { Price } from '/models/Price.ts'

@RequestRoute("/price")
@Public
class PriceController {
  @Needs
  readonly priceService!: PriceService

  @GetRoute
  getPrice(): Price {
    return this.priceService.priceFor(20, 3)
  }
}
