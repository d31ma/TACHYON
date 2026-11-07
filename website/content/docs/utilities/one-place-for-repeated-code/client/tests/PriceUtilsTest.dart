@Test(PriceTestSetup, {"partition":"tests"})
class PriceUtilsTest {
  Future<void> testLabelShowsDollars() async {
    Assert.equals('\$12.50', PriceUtils.label(1250));
  }

  Future<void> testBasketUsesTheUtility() async {
    final basket = BasketService();
    Assert.equals('\$12.50', basket.totalLabel());
  }
}
