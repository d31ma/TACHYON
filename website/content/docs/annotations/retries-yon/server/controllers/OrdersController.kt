import models.OrderReport

@RequestRoute("/orders")
@Public
class OrdersController {
    @Needs
    val orders: OrdersService

    @GetRoute
    fun getOrders(): OrderReport {
        val id = "A-1"
        val submitted = orders.submit(id)
        val report = orders.report(id)
        return OrderReport(submitted, report)
    }
}
