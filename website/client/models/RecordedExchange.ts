import type { RecordedRequest } from '/models/RecordedRequest.ts'
import type { RecordedResponse } from '/models/RecordedResponse.ts'

export type RecordedExchange = {
  request: RecordedRequest
  response: RecordedResponse
}
