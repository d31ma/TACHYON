import type { Section } from '/models/Section.ts'
import type { SectionBlock } from '/models/SectionBlock.ts'
import type { SnippetFile } from '/models/SnippetFile.ts'

export interface ShownSection extends Section {
  anchor: string
  blocks: SectionBlock[]
  single?: SnippetFile[]
}
