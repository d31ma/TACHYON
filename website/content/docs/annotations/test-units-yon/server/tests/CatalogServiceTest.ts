@Test(CatalogTestSetup, {"partition":"tests"})
@UseProfiles("test")
export default class CatalogServiceTest {
  @Fake
  catalogCounterHelper!: CatalogCounterHelper;

  async testDoublesTheCount(): Promise<void> {
    this.catalogCounterHelper = { count: () => 4 } as CatalogCounterHelper;
    const service = this.construct(CatalogService);
    Assert.equals(8, service.total());
  }

  async testHasAContext(): Promise<void> {
    Assert.isTrue(Boolean(this.behaviors), 'the case has a context');
  }
}
