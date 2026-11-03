@Test(PriceTestSetup)
class PriceUtilsTest {
  Future<void> testAddsTheTax() async {
    Assert.equals(1200, PriceUtils.withTax(1000));
  }

  @ValueSource([0, 5, 1000])
  Future<void> testNeverLowersAPrice(int cents) async {
    assert(PriceUtils.withTax(cents) >= cents, 'the tax never lowers a price');
  }

  Future<void> testRefusesANegativePrice() async {
    final error = await Assert.throws(() => PriceUtils.withTax(-1));
    Assert.equals('A price is never negative.', (error as ArgumentError).message);
  }
}
