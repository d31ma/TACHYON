@Test(CatalogTestSetup, {"partition":"tests"})
@UseProfiles("test")
export default class CatalogServiceTest {
  async testUsesAClientRole(): Promise<void> {
    const service = new CatalogService();
    if (service.double(4) !== 8) throw Error('expected 8');
  }

  async testHasAContext(): Promise<void> {
    if (!this.behaviors) throw Error('no context');
  }
}
