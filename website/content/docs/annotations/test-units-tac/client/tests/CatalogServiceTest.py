@Test(CatalogTestSetup, {"partition":"tests"})
@UseProfiles("test")
class CatalogServiceTest:
    async def test_uses_a_client_role(self) -> None:
        Assert.equals(8, CatalogService().double(4))

    async def test_has_a_context(self) -> None:
        Assert.is_true(self.behaviors is not None, 'the case has a context')
