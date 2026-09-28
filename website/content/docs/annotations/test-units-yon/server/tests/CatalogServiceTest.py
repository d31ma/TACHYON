@Test(CatalogTestSetup, {"partition":"tests"})
@UseProfiles("test")
class CatalogServiceTest:
    async def test_opens_a_context(self):
        if self.behaviors is None:
            raise Exception('no context')

    def twice(self, value):
        return value * 2

    async def test_doubles_a_value(self):
        if self.twice(4) != 8:
            raise Exception('expected 8')
