import models.OrderList

@RequestRoute('/orders')
@Public
class OrdersController {
    @GetRoute
    OrderList getOrders() {
        return new OrderList(OrderService.list())
    }
}
