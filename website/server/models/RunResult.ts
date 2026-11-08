import type { RunResponse } from '/models/RunResponse.ts'

export interface RunResult {
  outcome: string
  durationMs: number
  response?: RunResponse
}
