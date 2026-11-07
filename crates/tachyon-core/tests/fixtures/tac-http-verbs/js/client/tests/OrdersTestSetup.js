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
    return {
      method: request.method,
      path: request.path,
      header: Object.values(request.headers)[0],
      item: request.body ? request.body.item : 'none',
    };
  }
}
