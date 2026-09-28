@Test(CatalogTestSetup, {"partition":"tests"})
@UseProfiles("test")
class CatalogServiceTest {
  Future<void> testUsesAService() async {
    final service = CatalogService();
    if (service.double(4) != 8) throw StateError('expected 8');
  }

  Future<void> testHasAContext() async {
    if (behaviors == null) throw StateError('no context');
  }
}
