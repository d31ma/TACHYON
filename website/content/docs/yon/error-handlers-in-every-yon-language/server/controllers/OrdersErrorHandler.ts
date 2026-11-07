import type { Problem } from '/models/Problem.ts'

@ErrorHandler
class OrdersErrorHandler {
  @HandlesError
  @ResponseStatus(404)
  missing(error: OrderNotFound): Problem {
    return { code: 'order-not-found' }
  }
}
