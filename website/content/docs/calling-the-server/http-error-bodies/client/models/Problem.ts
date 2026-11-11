import type { ProblemEntry } from '/models/ProblemEntry.ts'

export interface Problem {
  title: string
  status: number
  errors: ProblemEntry[]
}
