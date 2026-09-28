@Test(PriceTestSetup, {"partition":"tests"})
class PriceUtilsTest:
    async def test_label_shows_dollars(self):
        if PriceUtils.label(1250) != "$12.50":
            raise Exception("expected $12.50")
