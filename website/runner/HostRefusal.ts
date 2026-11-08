import type { HostProblem } from './models/HostProblem.ts'

export class HostRefusal extends Error {
  constructor(readonly status: number, readonly problem: HostProblem) {
    super(problem.message)
  }
}
