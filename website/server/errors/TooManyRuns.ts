import type { Problem } from '/models/Problem.ts'

export default class TooManyRuns extends Error {
  readonly problem: Problem

  constructor(problem: Problem) {
    super(problem.message)
    this.problem = problem
  }
}
