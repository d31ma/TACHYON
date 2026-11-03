import type { Order } from '/models/Order.ts'
import type { SentRequest } from '/models/SentRequest.ts'

@HTTPClient({"path":"/api","adapter":"orders"})
export default class OrdersClient {
  @PostRequest("/orders")
  async create(@RequestBody order: Order, @RequestHeader("X-Tenant") tenant: string): Promise<SentRequest> {}

  @PutRequest("/orders/{id}")
  async replace(@PathVariable("id") id: string, @RequestBody order: Order, @RequestHeader("X-Tenant") tenant: string): Promise<SentRequest> {}

  @PatchRequest("/orders/{id}")
  async change(@PathVariable("id") id: string, @RequestBody change: Order, @RequestHeader("X-Tenant") tenant: string): Promise<SentRequest> {}

  @DeleteRequest("/orders/{id}")
  async remove(@PathVariable("id") id: string, @RequestHeader("X-Reason") reason: string): Promise<SentRequest> {}
}
