import type { SentRequest } from '/models/SentRequest.ts'

@Service
export default class OrderService {
  @Needs
  readonly ordersClient!: OrdersClient;

  async create(): Promise<SentRequest> {
    return this.ordersClient.create({ item: 'anvil' }, 'west');
  }

  async replace(): Promise<SentRequest> {
    return this.ordersClient.replace('a-7', { item: 'hammer' }, 'north');
  }

  async change(): Promise<SentRequest> {
    return this.ordersClient.change('a-7', { item: 'nails' }, 'south');
  }

  async remove(): Promise<SentRequest> {
    return this.ordersClient.remove('a-7', 'duplicate');
  }
}
