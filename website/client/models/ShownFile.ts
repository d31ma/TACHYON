import type { SnippetFile } from '/models/SnippetFile.ts'

export interface ShownFile extends SnippetFile {
  language: string
}
