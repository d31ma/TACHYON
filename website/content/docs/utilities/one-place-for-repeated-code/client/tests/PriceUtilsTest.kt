@Test(PriceTestSetup::class, {"partition":"tests"})
class PriceUtilsTest {
    fun testLabelShowsDollars() {
        Assert.equals("$12.50", PriceUtils.label(1250))
    }

    fun testBasketUsesTheUtility() {
        Assert.equals("$12.50", BasketService().totalLabel())
    }
}
