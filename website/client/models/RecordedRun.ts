import type { RecordedExchange } from '/models/RecordedExchange.ts'

export type RecordedRun = {
  schema: number
  exchanges: RecordedExchange[]
}
