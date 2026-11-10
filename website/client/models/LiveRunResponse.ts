import type { RecordedResponse } from '/models/RecordedResponse.ts'

export type LiveRunResponse = RecordedResponse & {
  truncated: boolean
}
