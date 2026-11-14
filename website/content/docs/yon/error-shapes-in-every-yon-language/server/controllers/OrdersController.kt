@RequestRoute("/orders")
@Allow(Admin::class)
class OrdersController {
    @Needs
    val orders: OrderService

    @GetRoute
    fun getOrders(): List<String> {
        throw IllegalStateException("the order store is down")
    }

    @PostRoute
    fun postOrder(@FromBody order: Any?): Any? = orders.accept(order)
}
