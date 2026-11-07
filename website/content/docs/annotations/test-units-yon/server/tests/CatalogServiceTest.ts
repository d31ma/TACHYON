@Test(CatalogTestSetup, {"partition":"tests"})
@UseProfiles("test")
export default class CatalogServiceTest {
  @Fake
  catalogCounterDelegate!: CatalogCounterDelegate;

  async testDoublesTheCount(): Promise<void> {
    this.catalogCounterDelegate = { count: () => 4 } as CatalogCounterDelegate;
    const service = this.construct(CatalogService);
    Assert.equals(8, service.total());
  }

  async testHasAContext(): Promise<void> {
    Assert.isTrue(Boolean(this.behaviors), 'the case has a context');
  }
}
