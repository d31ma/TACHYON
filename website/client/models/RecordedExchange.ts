import type { RecordedRequest } from '/models/RecordedRequest.ts'
import type { RecordedResponse } from '/models/RecordedResponse.ts'

export interface RecordedExchange {
  request: RecordedRequest
  response: RecordedResponse
}
