import type { SnippetFile } from '/models/SnippetFile.ts'

export interface FeatureEntry {
  id: string
  title: string
  summary: string
  group: string
  files?: SnippetFile[]
}
