@Test(PriceTestSetup)
export default class PriceUtilsTest {
  async testAddsTheTax(): Promise<void> {
    Assert.equals(1200, PriceUtils.withTax(1000))
  }

  @ValueSource([0, 5, 1000])
  async testNeverLowersAPrice(cents: number): Promise<void> {
    Assert.isTrue(PriceUtils.withTax(cents) >= cents, 'the tax never lowers a price')
  }

  async testRefusesANegativePrice(): Promise<void> {
    const error = Assert.throws(() => PriceUtils.withTax(-1)) as Error
    Assert.equals('A price is never negative.', error.message)
  }
}
