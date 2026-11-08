import type { LiveRunResponse } from '/models/LiveRunResponse.ts'

export interface LiveRunResult {
  outcome: string
  durationMs: number
  response?: LiveRunResponse
}
