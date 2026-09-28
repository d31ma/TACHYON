@Test(PriceTestSetup)
export default class PriceUtilsTest {
  async testAddsTheTax() {
    Assert.equals(1200, PriceUtils.withTax(1000))
  }

  @ValueSource([0, 5, 1000])
  async testNeverLowersAPrice(cents) {
    Assert.isTrue(PriceUtils.withTax(cents) >= cents, 'the tax never lowers a price')
  }

  async testRefusesANegativePrice() {
    const error = Assert.throws(() => PriceUtils.withTax(-1))
    Assert.equals('A price is never negative.', error.message)
  }
}
