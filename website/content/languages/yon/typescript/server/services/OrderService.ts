import type { Order } from '/models/Order.ts';
@Service
export class OrderService {
  list(): Order[] {
    return [{ sku: 'anvil', level: 4 }]
  }
}
