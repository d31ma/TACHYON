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
}
