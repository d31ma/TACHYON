@Test(CatalogTestSetup, {"partition":"tests"})
@UseProfiles("test")
export default class CatalogServiceTest {
  async testUsesAClientRole() {
    const service = new CatalogService();
    if (service.double(4) !== 8) throw Error('expected 8');
  }

  async testHasAContext() {
    if (!this.behaviors) throw Error('no context');
  }
}
