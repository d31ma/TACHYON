import type { Problem } from '/models/Problem.ts'

@ErrorHandler
export class OrdersErrorHandler {
  @HandlesError
  signInNeeded(error: NotSignedIn): Problem {
    return { code: 'not_signed_in', message: '[MS0001]: Sign in again.' }
  }

  @HandlesError
  notAllowed(error: RoleMissing): Problem {
    return { code: 'forbidden', message: '[MS0002]: You may not do this.' }
  }

  @HandlesError
  identityDown(error: IdentityCheckFailed): Problem {
    return { code: 'identity_failed', message: '[MS0003]: Try again later.' }
  }

  @HandlesError
  badRequest(error: RequestRejected): Problem {
    return { code: 'invalid_request', message: `[MS0004]: ${error.part}.${error.field}` }
  }

  @HandlesError
  tooLarge(error: BodyTooLarge): Problem {
    return { code: 'too_large', message: `[MS0005]: ${error.limitBytes}` }
  }

  @HandlesError
  brokenRules(error: CheckRejected): Problem {
    return { code: 'invalid_input', message: `[MS0006]: ${this.rules(error.violations)}` }
  }

  @HandlesError
  unexpected(error: UnexpectedError): Problem {
    return { code: 'unexpected', message: '[MS0000]: Something went wrong.' }
  }

  private rules(violations: BrokenRule[]): string {
    return violations.map((violation) => `${violation.field}:${violation.rule}`).join(',')
  }
}
