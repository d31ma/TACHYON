import { DocumentationUtils } from '/shared/utils/DocumentationUtils.js'
import type { Guide } from '/models/Guide.ts'
import type { Section } from '/models/Section.ts'
import type { SectionBlock } from '/models/SectionBlock.ts'
import type { TopicContents } from '/models/TopicContents.ts'
import type { ShownSection } from '/models/ShownSection.ts'
import type { BeforeLink } from '/models/BeforeLink.ts'
import type { SnippetFile } from '/models/SnippetFile.ts'

@Service
export default class TopicService {
  private static NOT_FOUND: Guide = {
    title: 'Topic not found',
    summary: 'That guide does not exist. Pick one from the list.',
    sections: [],
  }

  private static SEPARATOR: Record<string, string> = { true: '.', false: ', ' }

  private static anchorFor(heading: string): string {
    return `#${DocumentationUtils.slugify(heading)}`
  }

  private static blocks(body: string = ''): SectionBlock[] {
    return TopicService.blocksOf(TopicService.paragraphs(body))
  }

  get slug(): string {
    return TopicService.slugOf(TopicService.currentPath())
  }

  path(): string {
    return TopicService.currentPath()
  }

  get source(): string {
    return this.sourceOf(DocumentationUtils.guide(this.slug))
  }

  entry(): Guide {
    return this.orNotFound(DocumentationUtils.guide(this.slug))
  }

  sections(): ShownSection[] {
    return TopicService.shownSections(this.entry().sections)
  }

  before(): BeforeLink[] {
    return TopicService.linksTo(TopicService.knownGuides(this.readFirst()))
  }

  contents(): TopicContents[] {
    return TopicService.contentsOf(this.entry().sections)
  }

  words(): number {
    return TopicService.wordsIn(this.entry())
  }

  @OnStart
  initialize(): void {
    this.followRedirect(this.redirectTarget())
    document.title = `${this.entry().title} — Tachyon docs`
  }

  private static paragraphs(body: string): string[] {
    return body.split('\n\n').filter((part) => part.trim() !== '')
  }

  private static blocksOf(parts: string[]): SectionBlock[] {
    return parts.map(TopicService.blockOf)
  }

  private static blockOf(part: string): SectionBlock {
    if (!TopicService.isTable(part)) return { text: part }
    return TopicService.tableOf(part.trim().split('\n'))
  }

  private static isTable(part: string): boolean {
    return /^\s*\|[^\n]*(\n\s*\|[^\n]*){2,}$/.test(part.trim())
  }

  private static tableOf(lines: string[]): SectionBlock {
    const [header, , ...rows] = lines
    return { header: TopicService.cellsOf(header), rows: rows.map(TopicService.cellsOf) }
  }

  private static cellsOf(line: string): string[] {
    return line.trim().replace(/^\|/, '').replace(/\|$/, '').trim().split(/\s*\|\s*/)
  }

  private static shownSections(sections: Section[]): ShownSection[] {
    return sections.map(TopicService.shownSection)
  }

  private static shownSection(section: Section): ShownSection {
    return {
      ...section,
      axis: TopicService.axisOf(section),
      anchor: DocumentationUtils.slugify(section.heading),
      blocks: TopicService.blocks(section.body),
      single: TopicService.singleOf(section.code, TopicService.languageOf(section)),
    }
  }

  private static axisOf(section: Section): string {
    return section.axis ?? 'platform'
  }

  private static singleOf(code: string | undefined, name: string): SnippetFile[] | undefined {
    if (!code) return undefined
    return [{ name, code }]
  }

  private static languageOf(section: Section): string {
    return section.language ?? 'example'
  }

  private static knownGuides(slugs: string[]): string[] {
    return slugs.filter(DocumentationUtils.guide)
  }

  private static linksTo(slugs: string[]): BeforeLink[] {
    return slugs.map((slug, index) => ({
      title: TopicService.titleOf(slug),
      path: `/docs/${slug}`,
      separator: TopicService.SEPARATOR[String(index === slugs.length - 1)],
    }))
  }

  private static contentsOf(sections: Section[]): TopicContents[] {
    return sections.map((section) => ({
      heading: section.heading,
      anchor: TopicService.anchorFor(section.heading),
    }))
  }

  private static wordsIn(entry: Guide): number {
    const texts = [entry.summary, ...entry.sections.map(TopicService.proseOf)]
    return texts.join(' ').split(/\s+/).filter(Boolean).length
  }

  private static proseOf(section: Section): string {
    return `${section.summary ?? ''} ${section.body ?? ''}`
  }

  private sourceOf(found: Guide | undefined): string {
    if (!found) return 'website/content/docs/page.json'
    return `website/content/docs/${this.slug}/topic.json`
  }

  private orNotFound(found: Guide | undefined): Guide {
    return found ?? TopicService.NOT_FOUND
  }

  private readFirst(): string[] {
    return this.entry().before ?? []
  }

  private redirectTarget(): string | undefined {
    if (DocumentationUtils.guide(this.slug)) return undefined
    return DocumentationUtils.redirect(this.slug, location.hash)
  }

  private followRedirect(target: string | undefined): void {
    if (target) location.replace(target)
  }

  private static currentPath(): string {
    return location.pathname.replace(/(.)\/+$/, '$1')
  }

  private static slugOf(path: string): string {
    return path.split('/').filter(Boolean).pop() ?? ''
  }

  private static titleOf(slug: string): string {
    return DocumentationUtils.guide(slug)?.title ?? slug
  }
}
