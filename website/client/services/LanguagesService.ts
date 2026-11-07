import type { LanguageEntry } from '/models/LanguageEntry.ts'
import type { SnippetFile } from '/models/SnippetFile.ts'

@Service
@NeedsAll
export default class LanguagesService {
  entries!: LanguageEntry[]

  handlers(): SnippetFile[] {
    return this.entries.flatMap(entry => entry.files)
  }
}
