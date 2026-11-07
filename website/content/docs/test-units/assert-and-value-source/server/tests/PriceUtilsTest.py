@Test(PriceTestSetup)
class PriceUtilsTest:
    async def test_adds_the_tax(self):
        Assert.equals(1200, PriceUtils.with_tax(1000))

    @ValueSource([0, 5, 1000])
    async def test_never_lowers_a_price(self, cents):
        assert PriceUtils.with_tax(cents) >= cents, 'the tax never lowers a price'

    async def test_refuses_a_negative_price(self):
        error = Assert.throws(lambda: PriceUtils.with_tax(-1))
        Assert.equals('A price is never negative.', str(error))
