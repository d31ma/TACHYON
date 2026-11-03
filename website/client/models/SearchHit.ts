import type { SearchRow } from '/models/SearchRow.ts'

export interface SearchHit {
  row: SearchRow
  rank: number
}
