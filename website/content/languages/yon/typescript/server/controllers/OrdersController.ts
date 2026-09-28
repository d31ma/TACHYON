import type { OrderList } from '/models/OrderList.ts'

@RequestRoute("/orders")
@Public
class OrdersController {
  @GetRoute
  getOrders(): OrderList {
    return { orders: new OrderService().list() }
  }
}
