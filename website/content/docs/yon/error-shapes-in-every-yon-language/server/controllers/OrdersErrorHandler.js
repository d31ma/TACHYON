/** @import { Problem } from '/models/Problem.ts' */
@ErrorHandler
export class OrdersErrorHandler {
  /**
   * @param {NotSignedIn} error
   * @returns {Problem}
   */
  @HandlesError
  signInNeeded(error) {
    return { code: 'not_signed_in', message: '[MS0001]: Sign in again.' }
  }

  /**
   * @param {RoleMissing} error
   * @returns {Problem}
   */
  @HandlesError
  notAllowed(error) {
    return { code: 'forbidden', message: '[MS0002]: You may not do this.' }
  }

  /**
   * @param {IdentityCheckFailed} error
   * @returns {Problem}
   */
  @HandlesError
  identityDown(error) {
    return { code: 'identity_failed', message: '[MS0003]: Try again later.' }
  }

  /**
   * @param {RequestRejected} error
   * @returns {Problem}
   */
  @HandlesError
  badRequest(error) {
    return { code: 'invalid_request', message: `[MS0004]: ${error.part}.${error.field}` }
  }

  /**
   * @param {BodyTooLarge} error
   * @returns {Problem}
   */
  @HandlesError
  tooLarge(error) {
    return { code: 'too_large', message: `[MS0005]: ${error.limitBytes}` }
  }

  /**
   * @param {CheckRejected} error
   * @returns {Problem}
   */
  @HandlesError
  brokenRules(error) {
    return { code: 'invalid_input', message: `[MS0006]: ${this._rules(error.violations)}` }
  }

  /**
   * @param {UnexpectedError} error
   * @returns {Problem}
   */
  @HandlesError
  unexpected(error) {
    return { code: 'unexpected', message: '[MS0000]: Something went wrong.' }
  }

  /**
   * @param {BrokenRule[]} violations
   * @returns {string}
   */
  _rules(violations) {
    return violations.map((violation) => `${violation.field}:${violation.rule}`).join(',')
  }
}
