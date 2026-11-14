from typing import Annotated


@request_route("/orders")
@allow(Admin)
class OrdersController:
    order_service: Annotated[OrderService, needs]

    @get_route
    def get_orders(self) -> list[str]:
        raise OSError("the order store is down")

    @post_route
    def post_order(self, order: Annotated[dict, from_body]) -> dict:
        return self.order_service.accept(order)
