@Test(PriceTestSetup, {"partition":"tests"})
export default class PriceUtilsTest {
  async testLabelShowsDollars() {
    if (PriceUtils.label(1250) !== '$12.50') throw Error('expected $12.50')
  }

  async testBasketUsesTheUtility() {
    const basket = new BasketService()
    if (basket.totalLabel() !== '$12.50') throw Error('expected $12.50')
  }
}
