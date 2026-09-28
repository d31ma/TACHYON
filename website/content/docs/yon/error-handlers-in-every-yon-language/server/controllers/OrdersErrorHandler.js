/** @import { Problem } from '/models/Problem.ts' */
@ErrorHandler
class OrdersErrorHandler {
  /**
   * @param {OrderNotFound} error
   * @returns {Problem}
   */
  @HandlesError
  @ResponseStatus(404)
  missing(error) {
    return { code: 'order-not-found' }
  }
}
