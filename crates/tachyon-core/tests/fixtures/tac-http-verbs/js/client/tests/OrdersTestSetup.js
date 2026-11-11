@TestSetup
export default class OrdersTestSetup {
  static configure() {
    return { http: { orders: { exchange: OrdersTestSetup.exchange } } };
  }

  /**
   * @param {{ method: string, path: string, headers: Record<string, string>, body?: { item: string } }} request
   * @returns {SentRequest}
   */
  static exchange(request) {
    if (request.path === '/api/claims/a-7') throw OrdersTestSetup.claimTaken();
    return {
      method: request.method,
      path: request.path,
      header: Object.values(request.headers)[0],
      item: request.body ? request.body.item : 'none',
    };
  }

  static claimTaken() {
    return Object.assign(new Error('private fake text'), { code: 'HTTP_STATUS', status: 409, body: { reason: 'taken', holder: 'south' } });
  }
}
