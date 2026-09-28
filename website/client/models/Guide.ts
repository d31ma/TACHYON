import type { Section } from '/models/Section.ts'

export interface Guide {
  title: string
  summary: string
  before?: string[]
  sections: Section[]
}
