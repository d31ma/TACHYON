from models import Problem


@RequestRoute("/orders")
@Public
class OrdersController:
    @GetRoute
    def get_order(self) -> Problem:
        raise OrderNotFound("order 42")
