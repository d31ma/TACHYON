@Test(CatalogTestSetup, {"partition":"tests"})
@UseProfiles("test")
class CatalogServiceTest:
    async def test_opens_a_context(self):
        Assert.is_true(self.behaviors is not None, 'the case has a context')

    def twice(self, value):
        return value * 2

    async def test_doubles_a_value(self):
        Assert.equals(8, self.twice(4))
