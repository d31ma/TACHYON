[Test(typeof(PriceTestSetup), {"partition":"tests"})]
sealed class PriceUtilsTest {
    void testAddsTheTax() {
        Assert.equals(1200, PriceUtils.WithTax(1000));
    }

    [ValueSource([0, 5, 1000])]
    void testNeverLowersAPrice(long cents) {
        Assert.isTrue(PriceUtils.WithTax(cents) >= cents, "the tax never lowers a price");
    }

    void testRefusesANegativePrice() {
        var error = Assert.throws(() => PriceUtils.WithTax(-1));
        Assert.equals("A price is never negative.", error.Message);
    }
}
