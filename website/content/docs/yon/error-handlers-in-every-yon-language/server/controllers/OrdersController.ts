import type { Problem } from '/models/Problem.ts'

@RequestRoute("/orders")
@Public
class OrdersController {
  @GetRoute
  getOrder(): Problem {
    throw new OrderNotFound('order 42')
  }
}
