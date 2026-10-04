@Test(PriceTestSetup, {"partition":"tests"})
export default class PriceUtilsTest {
  async testLabelShowsDollars(): Promise<void> {
    Assert.equals('$12.50', PriceUtils.label(1250))
  }
}
