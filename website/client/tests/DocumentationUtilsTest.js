/** @import { DocumentationCatalogue } from '/models/DocumentationCatalogue.ts' */
/** @import { FeatureCatalogue } from '/models/FeatureCatalogue.ts' */
@Test(SiteTestSetup, {"partition":"tests"})
export default class DocumentationUtilsTest {
  /** @returns {DocumentationCatalogue} */
  documents() {
    return {
      order: ['utilities', 'pages'],
      topics: {
        utilities: {
          title: 'Utilities',
          summary: 'Stateless helpers that services share.',
          sections: [{ heading: 'One place for repeated code', body: 'A utility class holds static methods.' }],
        },
        pages: {
          title: 'Pages',
          summary: 'Each folder under pages is a route.',
          sections: [{ heading: 'Page utility imports', summary: 'A page imports a utility through its service.' }],
        },
      },
      redirects: { helpers: 'utilities', 'page-files': 'pages' },
    }
  }

  /** @returns {FeatureCatalogue} */
  features() {
    return {
      groups: ['Build'],
      features: [{ id: 'compiler', title: 'Compiler checks', summary: 'The compiler checks every view.', group: 'Build', files: [{ name: 'ty', code: 'ty build' }] }],
    }
  }

  async testSlugsAndGuidesComeFromTheCatalogue() {
    Assert.equals('one-place-for-repeated-code', DocumentationUtils.slugOf('One place for repeated code!'))
    Assert.isTrue(DocumentationUtils.guide(this.documents(), 'utilities') !== undefined, 'a guide is found by its slug')
    Assert.isTrue(DocumentationUtils.guide(this.documents(), 'no-such-guide') === undefined, 'an unknown slug has no guide')
    Assert.isTrue(DocumentationUtils.search('utilities', this.documents(), this.features(), 100).some((hit) => hit.kind === 'topic'), 'the guides are searched')
    Assert.isTrue(DocumentationUtils.search('compiler', this.documents(), this.features(), 100).some((hit) => hit.kind === 'feature'), 'the features are searched')
  }

  async testRedirectsFollowTheCatalogue() {
    Assert.equals('/docs/utilities', DocumentationUtils.redirect(this.documents(), 'helpers'))
    Assert.equals('/docs/pages#page-utility-imports', DocumentationUtils.redirect(this.documents(), 'helpers', '#page-utility-imports'), 'an anchor goes to the guide that holds it')
    Assert.isTrue(DocumentationUtils.redirect(this.documents(), 'no-such-guide') === undefined, 'an unknown slug is not redirected')
  }

  async testSearchKeepsTheMaximum() {
    Assert.equals(2, DocumentationUtils.search('util', this.documents(), this.features(), 2).length, 'search keeps the maximum')
    Assert.equals(3, DocumentationUtils.search('util', this.documents(), this.features()).length, 'the default maximum keeps every hit here')
  }

  async testSearchLinksEachHitToItsPage() {
    const hits = DocumentationUtils.search('compiler', this.documents(), this.features(), 100)
    Assert.isTrue(hits.filter((hit) => hit.kind === 'feature').every((hit) => hit.path.startsWith('/docs/features/')), 'a feature hit opens its feature page')
    const sections = DocumentationUtils.search('static methods', this.documents(), this.features(), 100)
    Assert.equals('/docs/utilities#one-place-for-repeated-code', sections[0]?.path, 'a section hit opens its place in the guide')
    Assert.equals('section', sections[0]?.kind)
  }

  async testATitleThatStartsWithTheTermRanksFirst() {
    const [first] = DocumentationUtils.search('utilities', this.documents(), this.features(), 100)
    Assert.isTrue(first.title.toLowerCase().startsWith('utilities'), first.title)
    Assert.equals(0, DocumentationUtils.search('u', this.documents(), this.features(), 100).length, 'a term shorter than the shortest term finds nothing')
  }
}
