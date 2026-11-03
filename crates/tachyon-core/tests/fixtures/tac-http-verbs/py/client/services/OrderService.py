from typing import Annotated

from models import SentRequest


@service
class OrderService:
    orders_client: Annotated[OrdersClient, needs]

    async def replace(self) -> SentRequest:
        return await self.orders_client.replace('a-7', {'item': 'hammer'}, 'north')

    async def change(self) -> SentRequest:
        return await self.orders_client.change('a-7', {'item': 'nails'}, 'south')

    async def remove(self) -> SentRequest:
        return await self.orders_client.remove('a-7', 'duplicate')
