@Test(OrdersTestSetup, {"partition":"tac-http-verbs"})
class OrderServiceTest {
  @Fake
  OrdersClient? ordersClient;

  Future<void> testPutSendsThePathTheHeaderAndTheBody() async {
    final sent = await construct<OrderService>(OrderService).replace();
    Assert.equals('PUT', sent.method);
    Assert.equals('/api/orders/a-7', sent.path);
    Assert.equals('north', sent.header);
    Assert.equals('hammer', sent.item);
  }

  Future<void> testPatchSendsThePathTheHeaderAndTheBody() async {
    final sent = await construct<OrderService>(OrderService).change();
    Assert.equals('PATCH', sent.method);
    Assert.equals('/api/orders/a-7', sent.path);
    Assert.equals('south', sent.header);
    Assert.equals('nails', sent.item);
  }

  Future<void> testDeleteSendsThePathAndTheHeaderWithoutABody() async {
    final sent = await construct<OrderService>(OrderService).remove();
    Assert.equals('DELETE', sent.method);
    Assert.equals('/api/orders/a-7', sent.path);
    Assert.equals('duplicate', sent.header);
    Assert.equals('none', sent.item);
  }
}
