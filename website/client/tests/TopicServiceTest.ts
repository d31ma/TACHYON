import type { SectionBlock } from '/models/SectionBlock.ts'

@Test(SiteTestSetup, {"partition":"tests"})
export default class TopicServiceTest {
  async testEntryIsTheGuideTheUrlNames() {
    history.pushState({}, '', '/docs/annotations')
    const topic = new TopicService()
    if (topic.slug !== 'annotations') throw Error(topic.slug)
    if (topic.path() !== '/docs/annotations') throw Error(topic.path())
    if (topic.entry().sections.length === 0) throw Error('the annotations guide has sections')
    if (topic.source !== 'website/content/docs/annotations/topic.json') throw Error(topic.source)
  }

  async testBeforeLinksTheTopicsToReadFirst() {
    history.pushState({}, '', '/docs/checking-values')
    const before = new TopicService().before()
    if (before.map((link: { path: string }) => link.path).join(',') !== '/docs/controllers,/docs/models') throw Error('the topics to read first, in order')
    if (before.map((link: { separator: string }) => link.separator).join('') !== ', .') throw Error('a comma between links and a full stop at the end')
    if (before[0].title !== 'Controllers') throw Error(before[0].title)
  }

  async testSectionsAndContentsShareAnchors() {
    history.pushState({}, '', '/docs/annotations')
    const topic = new TopicService()
    const sections = topic.sections()
    const contents = topic.contents()
    if (sections.length !== contents.length) throw Error('one contents entry per section')
    sections.forEach((section: { anchor: string, heading: string }, index: number) => {
      if (contents[index].anchor !== `#${section.anchor}`) throw Error(`${section.heading} anchors differently`)
    })
  }

  async testEverySectionNamesTheAxisItsSnippetNeeds() {
    history.pushState({}, '', '/docs/application-events')
    const sections = new TopicService().sections()
    if (!sections.some((section: { axis?: string }) => section.axis === 'language')) throw Error('a language section keeps its axis')
    if (sections.some((section: { axis?: string }) => section.axis !== 'platform' && section.axis !== 'language')) throw Error('every section names platform or language')
  }

  async testAMarkdownTableBecomesRowsAndOtherTextStaysParagraphs() {
    history.pushState({}, '', '/docs/cohesive-services')
    const blocks = new TopicService().sections().flatMap((section: { blocks: SectionBlock[] }) => section.blocks)
    const table = blocks.find((block: SectionBlock) => block.header)
    if (!table?.header?.length) throw Error('the table keeps its header')
    if (!table.rows?.[0]?.length) throw Error('the table keeps its rows')
    if (!blocks.some((block: SectionBlock) => block.text)) throw Error('other text stays paragraphs')
  }

  async testInitializeTitlesTheDocumentAfterTheGuide() {
    history.pushState({}, '', '/docs/nowhere')
    const topic = new TopicService()
    topic.initialize()
    if (document.title !== 'Topic not found — Tachyon docs') throw Error(document.title)
    if (topic.source !== 'website/content/docs/page.json') throw Error(topic.source)
  }
}
