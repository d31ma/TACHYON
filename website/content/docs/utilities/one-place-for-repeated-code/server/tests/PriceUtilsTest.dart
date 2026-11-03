@Test(PriceTestSetup, {"partition":"tests"})
class PriceUtilsTest {
  Future<void> testLabelShowsDollars() async {
    Assert.equals('\$12.50', PriceUtils.label(1250));
  }
}
