@Test(CatalogTestSetup, {"partition":"tests"})
@UseProfiles("test")
export default class CatalogServiceTest {
  @Fake
  catalogCounterHelper!: CatalogCounterHelper;

  async testDoublesTheCount(): Promise<void> {
    this.catalogCounterHelper = { count: () => 4 } as CatalogCounterHelper;
    const service = this.construct(CatalogService);
    if (service.total() !== 8) throw Error('expected 8');
  }

  async testHasAContext(): Promise<void> {
    if (!this.behaviors) throw Error('no context');
  }
}
