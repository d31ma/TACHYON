using Models;

[ErrorHandler]
class OrdersErrorHandler {
    [HandlesError]
    [ResponseStatus(404)]
    public Problem Missing(OrderNotFound error) {
        return new Problem("order-not-found");
    }
}
