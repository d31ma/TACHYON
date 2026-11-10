import type { Problem } from '/models/Problem.ts'

@ErrorHandler
export class SnippetRunErrorHandler {
  @HandlesError
  @ResponseStatus(400)
  invalid(error: InvalidRun): Problem {
    return error.problem
  }

  @HandlesError
  @ResponseStatus(404)
  missing(error: ExampleNotFound): Problem {
    return error.problem
  }

  @HandlesError
  @ResponseStatus(413)
  tooLarge(error: RunTooLarge): Problem {
    return error.problem
  }

  @HandlesError
  @ResponseStatus(429)
  tooMany(error: TooManyRuns): Problem {
    return error.problem
  }

  @HandlesError
  @ResponseStatus(503)
  busy(error: RunnerBusy): Problem {
    return error.problem
  }
}
