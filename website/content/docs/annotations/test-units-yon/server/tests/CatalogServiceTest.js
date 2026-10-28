@Test(CatalogTestSetup, {"partition":"tests"})
@UseProfiles("test")
export default class CatalogServiceTest {
  /** @type {CatalogCounterDelegate} */
  @Fake
  catalogCounterDelegate;

  async testDoublesTheCount() {
    this.catalogCounterDelegate = { count: () => 4 };
    const service = this.construct(CatalogService);
    Assert.equals(8, service.total());
  }

  async testHasAContext() {
    Assert.isTrue(Boolean(this.behaviors), 'the case has a context');
  }
}
