import models.Problem

@ErrorHandler
class OrdersErrorHandler {
    @HandlesError
    @ResponseStatus(404)
    fun missing(error: OrderNotFound): Problem {
        return Problem("order-not-found")
    }
}
