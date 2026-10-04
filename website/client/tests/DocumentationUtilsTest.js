// @ts-nocheck

@Test(SiteTestSetup, {"partition":"tests"})
export default class DocumentationUtilsTest {
  async testSlugsAndGuidesComeFromTheSiteData() {
    Assert.equals('one-place-for-repeated-code', DocumentationUtils.slugOf('One place for repeated code!'))
    Assert.isTrue(DocumentationUtils.guide('utilities') !== undefined, 'a guide is found by its slug')
    Assert.isTrue(DocumentationUtils.guide('no-such-guide') === undefined, 'an unknown slug has no guide')
    Assert.isTrue(DocumentationUtils.search('utilities', 100).some((hit) => hit.kind === 'topic'), 'the guides are searched')
    Assert.isTrue(DocumentationUtils.search('compiler', 100).some((hit) => hit.kind === 'feature'), 'the features are searched')
  }

  async testSearchAndRedirectReadTheSameData() {
    Assert.isTrue(DocumentationUtils.search('utility', 3).length <= 3, 'search keeps the maximum')
    Assert.isTrue(DocumentationUtils.redirect('no-such-guide') === undefined, 'an unknown slug is not redirected')
  }

  async testSearchLinksEachHitToItsPage() {
    const hits = DocumentationUtils.search('compiler', 100)
    Assert.isTrue(hits.filter((hit) => hit.kind === 'feature').every((hit) => hit.path.startsWith('/docs/features/')), 'a feature hit opens its feature page')
    Assert.isTrue(hits.filter((hit) => hit.kind === 'section').every((hit) => hit.path.includes('#')), 'a section hit opens its place in the guide')
    Assert.isTrue(hits.some((hit) => hit.kind === 'section'), 'the sections of a guide are searched')
  }

  async testATitleThatStartsWithTheTermRanksFirst() {
    const [first] = DocumentationUtils.search('utilities', 100)
    Assert.isTrue(first.title.toLowerCase().startsWith('utilities'), first.title)
    Assert.equals(0, DocumentationUtils.search('u', 100).length, 'a term shorter than the shortest term finds nothing')
  }
}
