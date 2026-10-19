@Test(OrdersTestSetup, {"partition":"tac-http-verbs"})
export default class OrderServiceTest {
  @Fake
  ordersClient!: OrdersClient;

  service(): OrderService {
    this.ordersClient = Object.assign(new OrdersClient(), { behaviors: this.behaviors });
    return this.construct(OrderService);
  }

  async testPutSendsThePathTheHeaderAndTheBody(): Promise<void> {
    const sent = await this.service().replace();
    Assert.equals('PUT', sent.method);
    Assert.equals('/api/orders/a-7', sent.path);
    Assert.equals('north', sent.header);
    Assert.equals('hammer', sent.item);
  }

  async testPatchSendsThePathTheHeaderAndTheBody(): Promise<void> {
    const sent = await this.service().change();
    Assert.equals('PATCH', sent.method);
    Assert.equals('/api/orders/a-7', sent.path);
    Assert.equals('south', sent.header);
    Assert.equals('nails', sent.item);
  }

  async testDeleteSendsThePathAndTheHeaderWithoutABody(): Promise<void> {
    const sent = await this.service().remove();
    Assert.equals('DELETE', sent.method);
    Assert.equals('/api/orders/a-7', sent.path);
    Assert.equals('duplicate', sent.header);
    Assert.equals('none', sent.item);
  }
}
