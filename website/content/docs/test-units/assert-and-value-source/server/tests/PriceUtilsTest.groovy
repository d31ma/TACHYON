@Test(PriceTestSetup.class)
class PriceUtilsTest {
    void testAddsTheTax() {
        Assert.equals(1200, PriceUtils.withTax(1000))
    }

    @ValueSource([0, 5, 1000])
    void testNeverLowersAPrice(long cents) {
        assert PriceUtils.withTax(cents) >= cents : 'the tax never lowers a price'
    }

    void testRefusesANegativePrice() {
        Throwable error = Assert.throwsError({ PriceUtils.withTax(-1) })
        Assert.equals('A price is never negative.', error.message)
    }
}
