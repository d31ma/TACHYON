@Test(OrdersTestSetup, {"partition":"tac-http-verbs"})
export default class OrderServiceTest {
  /** @type {OrdersClient} */
  @Fake
  ordersClient;

  service() {
    this.ordersClient = Object.assign(new OrdersClient(), { behaviors: this.behaviors });
    return this.construct(OrderService);
  }

  async testPutSendsThePathTheHeaderAndTheBody() {
    const sent = await this.service().replace();
    Assert.equals('PUT', sent.method);
    Assert.equals('/api/orders/a-7', sent.path);
    Assert.equals('north', sent.header);
    Assert.equals('hammer', sent.item);
  }

  async testPatchSendsThePathTheHeaderAndTheBody() {
    const sent = await this.service().change();
    Assert.equals('PATCH', sent.method);
    Assert.equals('/api/orders/a-7', sent.path);
    Assert.equals('south', sent.header);
    Assert.equals('nails', sent.item);
  }

  async testDeleteSendsThePathAndTheHeaderWithoutABody() {
    const sent = await this.service().remove();
    Assert.equals('DELETE', sent.method);
    Assert.equals('/api/orders/a-7', sent.path);
    Assert.equals('duplicate', sent.header);
    Assert.equals('none', sent.item);
  }

  async testErrorBodyIsReadIntoAModel() {
    const error = await Assert.throws(() => this.service().claim());
    /** @type {ClaimConflict | null} */
    const conflict = Behaviors.errorBody(error);
    Assert.equals('HTTP_STATUS', error.code);
    Assert.equals('The HTTP request failed.', error.message);
    Assert.equals(409, Behaviors.errorStatus(error));
    Assert.equals({ reason: 'taken', holder: 'south' }, conflict);
  }
}
