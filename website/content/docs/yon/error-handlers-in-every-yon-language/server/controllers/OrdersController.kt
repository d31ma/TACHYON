import models.Problem

@RequestRoute("/orders")
@Public
class OrdersController {
    @GetRoute
    fun getOrder(): Problem {
        throw OrderNotFound("order 42")
    }
}
