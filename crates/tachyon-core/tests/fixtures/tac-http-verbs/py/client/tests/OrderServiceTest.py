from typing import Annotated


@Test(OrdersTestSetup, {"partition":"tac-http-verbs"})
class OrderServiceTest:
    orders_client: Annotated[OrdersClient, fake]

    def service(self) -> OrderService:
        self.orders_client = OrdersClient()
        self.orders_client.behaviors = self.behaviors
        return OrderService(self.orders_client)

    async def test_put_sends_the_path_the_header_and_the_body(self) -> None:
        sent = await self.service().replace()
        Assert.equals('PUT', sent.method)
        Assert.equals('/api/orders/a-7', sent.path)
        Assert.equals('north', sent.header)
        Assert.equals('hammer', sent.item)

    async def test_patch_sends_the_path_the_header_and_the_body(self) -> None:
        sent = await self.service().change()
        Assert.equals('PATCH', sent.method)
        Assert.equals('/api/orders/a-7', sent.path)
        Assert.equals('south', sent.header)
        Assert.equals('nails', sent.item)

    async def test_delete_sends_the_path_and_the_header_without_a_body(self) -> None:
        sent = await self.service().remove()
        Assert.equals('DELETE', sent.method)
        Assert.equals('/api/orders/a-7', sent.path)
        Assert.equals('duplicate', sent.header)
        Assert.equals('none', sent.item)
