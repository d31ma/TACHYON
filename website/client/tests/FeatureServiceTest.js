// @ts-nocheck

@Test(SiteTestSetup, {"partition":"tests"})
export default class FeatureServiceTest {
  async testEntryIsTheFeatureTheUrlNames() {
    history.pushState({}, '', '/docs/features/routing')
    const feature = new FeatureService()
    Assert.equals('routing', feature.id)
    Assert.equals('/docs/features/routing', feature.path())
    Assert.equals('routing', feature.entry().id, 'entry is the routing feature')
    Assert.isTrue(feature.files().length > 0, 'the routing feature ships its files')
    Assert.isTrue(feature.groupAnchor().startsWith('/docs/features#'), feature.groupAnchor())
    Assert.equals('website/content/features/routing/feature.json', feature.source)
  }

  async testAnUnknownFeatureIsNotFoundRatherThanBroken() {
    history.pushState({}, '', '/docs/features/no-such-feature')
    const feature = new FeatureService()
    Assert.equals('Feature not found', feature.entry().title, 'an unknown id is the not-found entry')
    Assert.equals([], feature.files(), 'and has no files')
    Assert.equals('website/content/features/page.json', feature.source)
  }

  async testInitializeTitlesTheDocumentAfterTheFeature() {
    history.pushState({}, '', '/docs/features/routing')
    const feature = new FeatureService()
    feature.initialize()
    Assert.equals(`${feature.entry().title} — Tachyon docs`, document.title)
  }
}
