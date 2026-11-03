from typing import Annotated
from models import OrderReport


@request_route("/orders")
@public
class OrdersController:
    orders_service: Annotated[OrdersService, needs]

    @get_route
    async def get_orders(self) -> OrderReport:
        id = 'A-1'
        submitted = await self.orders_service.submit(id)
        report = await self.orders_service.report(id)
        return OrderReport(submitted, report)
