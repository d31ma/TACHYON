import type { SearchResult } from '/models/SearchResult.ts'

export interface SearchRow extends SearchResult {
  text: string
}
