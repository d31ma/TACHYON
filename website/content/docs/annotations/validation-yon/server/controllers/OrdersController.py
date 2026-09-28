from typing import Annotated

@request_route("/orders")
@public
class OrdersController:
    orders: Annotated[OrderService, needs]
    @post_route
    def post_orders(self, order: Annotated[dict, from_body]) -> dict:
        return self.orders.accept(order)
