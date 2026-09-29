@Test(PriceTestSetup::class)
class PriceUtilsTest {
    fun testAddsTheTax() {
        Assert.equals(1200, PriceUtils.withTax(1000))
    }

    @ValueSource([0, 5, 1000])
    fun testNeverLowersAPrice(cents: Long) {
        check(PriceUtils.withTax(cents) >= cents) { "the tax never lowers a price" }
    }

    fun testRefusesANegativePrice() {
        val error = Assert.throws { PriceUtils.withTax(-1) }
        Assert.equals("A price is never negative.", error.message)
    }
}
