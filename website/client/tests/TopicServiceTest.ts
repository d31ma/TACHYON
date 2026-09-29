import type { SectionBlock } from '/models/SectionBlock.ts'

@Test(SiteTestSetup, {"partition":"tests"})
export default class TopicServiceTest {
  async testEntryIsTheGuideTheUrlNames() {
    history.pushState({}, '', '/docs/annotations')
    const topic = new TopicService()
    Assert.equals('annotations', topic.slug)
    Assert.equals('/docs/annotations', topic.path())
    Assert.isTrue(topic.entry().sections.length > 0, 'the annotations guide has sections')
    Assert.equals('website/content/docs/annotations/topic.json', topic.source)
  }

  async testBeforeLinksTheTopicsToReadFirst() {
    history.pushState({}, '', '/docs/checking-values')
    const before = new TopicService().before()
    Assert.equals('/docs/controllers,/docs/models', before.map((link: { path: string }) => link.path).join(','), 'the topics to read first, in order')
    Assert.equals(', .', before.map((link: { separator: string }) => link.separator).join(''), 'a comma between links and a full stop at the end')
    Assert.equals('Controllers', before[0].title)
  }

  async testSectionsAndContentsShareAnchors() {
    history.pushState({}, '', '/docs/annotations')
    const topic = new TopicService()
    const sections = topic.sections()
    const contents = topic.contents()
    const anchors = sections.map((section: { anchor: string }) => `#${section.anchor}`)
    Assert.equals(anchors, contents.map((entry: { anchor: string }) => entry.anchor), 'one contents entry per section, with the same anchor')
  }

  async testEverySectionNamesTheAxisItsSnippetNeeds() {
    history.pushState({}, '', '/docs/application-events')
    const sections = new TopicService().sections()
    Assert.isTrue(sections.some((section: { axis?: string }) => section.axis === 'language'), 'a language section keeps its axis')
    Assert.isTrue(sections.every((section: { axis?: string }) => section.axis === 'platform' || section.axis === 'language'), 'every section names platform or language')
  }

  async testAMarkdownTableBecomesRowsAndOtherTextStaysParagraphs() {
    history.pushState({}, '', '/docs/cohesive-services')
    const blocks = new TopicService().sections().flatMap((section: { blocks: SectionBlock[] }) => section.blocks)
    const table = blocks.find((block: SectionBlock) => block.header)
    Assert.isTrue(Boolean(table?.header?.length), 'the table keeps its header')
    Assert.isTrue(Boolean(table?.rows?.[0]?.length), 'the table keeps its rows')
    Assert.isTrue(blocks.some((block: SectionBlock) => Boolean(block.text)), 'other text stays paragraphs')
  }

  async testInitializeTitlesTheDocumentAfterTheGuide() {
    history.pushState({}, '', '/docs/nowhere')
    const topic = new TopicService()
    topic.initialize()
    Assert.equals('Topic not found — Tachyon docs', document.title)
    Assert.equals('website/content/docs/page.json', topic.source)
  }
}
