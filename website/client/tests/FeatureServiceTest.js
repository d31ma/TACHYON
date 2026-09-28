// @ts-nocheck

@Test(SiteTestSetup, {"partition":"tests"})
export default class FeatureServiceTest {
  async testEntryIsTheFeatureTheUrlNames() {
    history.pushState({}, '', '/docs/features/routing')
    const feature = new FeatureService()
    if (feature.id !== 'routing') throw Error(feature.id)
    if (feature.path() !== '/docs/features/routing') throw Error(feature.path())
    if (feature.entry().id !== 'routing') throw Error('entry is the routing feature')
    if (feature.files().length === 0) throw Error('the routing feature ships its files')
    if (!feature.groupAnchor().startsWith('/docs/features#')) throw Error(feature.groupAnchor())
    if (feature.source !== 'website/content/features/routing/feature.json') throw Error(feature.source)
  }

  async testAnUnknownFeatureIsNotFoundRatherThanBroken() {
    history.pushState({}, '', '/docs/features/no-such-feature')
    const feature = new FeatureService()
    if (feature.entry().title !== 'Feature not found') throw Error('an unknown id is the not-found entry')
    if (feature.files().length !== 0) throw Error('and has no files')
    if (feature.source !== 'website/content/features/page.json') throw Error(feature.source)
  }

  async testInitializeTitlesTheDocumentAfterTheFeature() {
    history.pushState({}, '', '/docs/features/routing')
    const feature = new FeatureService()
    feature.initialize()
    if (document.title !== `${feature.entry().title} — Tachyon docs`) throw Error(document.title)
  }
}
