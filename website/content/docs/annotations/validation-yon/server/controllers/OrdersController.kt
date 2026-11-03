@RequestRoute("/orders")
@Public
class OrdersController {
    @Needs
    val orders: OrderService
    @PostRoute
    fun postOrders(@FromBody order: Any?): Any? = orders.accept(order)
}
