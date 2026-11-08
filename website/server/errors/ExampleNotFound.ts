import type { Problem } from '/models/Problem.ts'

export default class ExampleNotFound extends Error {
  readonly problem: Problem

  constructor(problem: Problem) {
    super(problem.message)
    this.problem = problem
  }
}
