import type { RecordedExchange } from '/models/RecordedExchange.ts'

export interface RecordedRun {
  schema: number
  exchanges: RecordedExchange[]
}
