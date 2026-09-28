from models import OrderList


@request_route("/orders")
@public
class OrdersController:
    @get_route
    def get_orders(self) -> OrderList:
        return OrderList(OrderService().list())
