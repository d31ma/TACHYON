@Test(CatalogTestSetup, {"partition":"tests"})
@UseProfiles("test")
export default class CatalogServiceTest {
  @Fake
  catalogCounterHelper;

  async testDoublesTheCount() {
    this.catalogCounterHelper = { count: () => 4 };
    const service = this.construct(CatalogService);
    if (service.total() !== 8) throw Error('expected 8');
  }

  async testHasAContext() {
    if (!this.behaviors) throw Error('no context');
  }
}
