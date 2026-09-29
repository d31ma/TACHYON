import type { Order } from '/models/Order.ts';
@Store
export class OrderStore {
  all(): Order[] {
    return [{ sku: 'anvil', level: 4 }]
  }
}
