@Test(CatalogTestSetup, {"partition":"tests"})
@UseProfiles("test")
export default class CatalogServiceTest {
  async testUsesAClientRole() {
    const service = new CatalogService();
    Assert.equals(8, service.double(4));
  }

  async testHasAContext() {
    Assert.isTrue(Boolean(this.behaviors), 'the case has a context');
  }
}
