@Test(PriceTestSetup, {"partition":"tests"})
class PriceUtilsTest:
    async def test_label_shows_dollars(self):
        Assert.equals("$12.50", PriceUtils.label(1250))

    async def test_basket_uses_the_utility(self):
        Assert.equals("$12.50", BasketService().total_label())
