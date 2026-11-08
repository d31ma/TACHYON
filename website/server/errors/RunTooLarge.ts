import type { Problem } from '/models/Problem.ts'

export default class RunTooLarge extends Error {
  readonly problem: Problem

  constructor(problem: Problem) {
    super(problem.message)
    this.problem = problem
  }
}
