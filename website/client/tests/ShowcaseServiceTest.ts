@Test(SiteTestSetup, {"partition":"tests"})
export default class ShowcaseServiceTest {
  async testTacIsShownFirst() {
    const showcase = new ShowcaseService()
    Assert.equals('tac', showcase.layer)
    Assert.isTrue(showcase.summary().startsWith('One component runs on every target.'), 'the Tac summary')
  }

  async testPickLayerChangesTheLayerAndItsSummary() {
    const showcase = new ShowcaseService()
    const tac = showcase.summary()
    showcase.pickLayer('yon')
    Assert.equals('yon', showcase.layer, 'pickLayer selects a layer')
    Assert.isTrue(showcase.summary() !== tac, 'each layer has its own summary')
  }
}
