import type { SnippetFile } from '/models/SnippetFile.ts'

export interface LanguageEntry {
  id: string
  name: string
  files: SnippetFile[]
}
