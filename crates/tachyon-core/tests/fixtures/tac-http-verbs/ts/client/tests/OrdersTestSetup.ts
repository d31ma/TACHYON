import type { Order } from '/models/Order.ts'
import type { SentRequest } from '/models/SentRequest.ts'

@TestSetup
export default class OrdersTestSetup {
  static configure(): Record<string, unknown> {
    return { http: { orders: { exchange: OrdersTestSetup.exchange } } };
  }

  static exchange(request: { method: string, path: string, headers: Record<string, string>, body?: Order }): SentRequest {
    return {
      method: request.method,
      path: request.path,
      header: Object.values(request.headers)[0],
      item: request.body ? request.body.item : 'none',
    };
  }
}
