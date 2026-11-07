@Test(CatalogTestSetup, {"partition":"tests"})
@UseProfiles("test")
class CatalogServiceTest {
  Future<void> testUsesAService() async {
    final service = CatalogService();
    Assert.equals(8, service.double(4));
  }

  Future<void> testHasAContext() async {
    Assert.isTrue(behaviors != null, 'a context');
  }
}
