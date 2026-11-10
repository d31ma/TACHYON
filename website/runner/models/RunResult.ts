import type { RunResponse } from './RunResponse.ts'

export interface RunResult {
  outcome: 'ok' | 'timeout'
  durationMs: number
  response?: RunResponse
}
