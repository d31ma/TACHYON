import models.Problem

@ErrorHandler
class OrdersErrorHandler {
    @HandlesError
    @ResponseStatus(404)
    Problem missing(OrderNotFound error) {
        return new Problem("order-not-found")
    }
}
