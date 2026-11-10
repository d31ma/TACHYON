import type { Problem } from '/models/Problem.ts'

export default class InvalidRun extends Error {
  readonly problem: Problem

  constructor(problem: Problem) {
    super(problem.message)
    this.problem = problem
  }
}
