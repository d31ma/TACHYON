from models import Problem


@ErrorHandler
class OrdersErrorHandler:
    @HandlesError
    @ResponseStatus(404)
    def missing(self, error: OrderNotFound) -> Problem:
        return Problem("order-not-found")
