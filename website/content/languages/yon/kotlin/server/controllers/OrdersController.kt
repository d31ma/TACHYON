import models.OrderList

@RequestRoute("/orders")
@Public
class OrdersController {
    @GetRoute
    fun getOrders(): OrderList = OrderList(OrderService.list())
}
