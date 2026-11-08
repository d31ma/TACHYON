import type { RecordedResponse } from '/models/RecordedResponse.ts'

export interface LiveRunResponse extends RecordedResponse {
  truncated: boolean
}
