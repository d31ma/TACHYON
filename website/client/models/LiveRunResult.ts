import type { LiveRunResponse } from '/models/LiveRunResponse.ts'

export type LiveRunResult = {
  outcome: string
  durationMs: number
  response?: LiveRunResponse
}
