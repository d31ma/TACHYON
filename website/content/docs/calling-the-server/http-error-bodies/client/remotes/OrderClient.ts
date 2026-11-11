import type { OrderInput } from '/models/OrderInput.ts'

@HTTPClient("/api")
export default class OrderClient {
  @PostRequest("/orders")
  async send(@RequestBody order: OrderInput): Promise<OrderInput | void> {}
}
