// @ts-nocheck

@Test(SiteTestSetup, {"partition":"tests"})
export default class DocumentationUtilsTest {
  async testSlugsAndGuidesComeFromTheSiteData() {
    Assert.equals('one-place-for-repeated-code', DocumentationUtils.slugify('One place for repeated code!'))
    Assert.isTrue(DocumentationUtils.guide('utilities') !== undefined, 'a guide is found by its slug')
    Assert.isTrue(DocumentationUtils.guide('no-such-guide') === undefined, 'an unknown slug has no guide')
    Assert.isTrue(DocumentationUtils.guides().length > 0, 'the guides are listed')
    Assert.isTrue(Array.isArray(DocumentationUtils.features()), 'the features are listed')
  }

  async testSearchAndRedirectReadTheSameData() {
    Assert.isTrue(DocumentationUtils.searchDocs('utility', 3).length <= 3, 'search keeps the maximum')
    Assert.isTrue(DocumentationUtils.redirect('no-such-guide') === undefined, 'an unknown slug is not redirected')
  }
}
