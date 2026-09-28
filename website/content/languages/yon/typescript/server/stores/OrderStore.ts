import type { Order } from '/models/Order';
@Store
export class OrderStore {
  all(): Order[] {
    return [{ sku: 'anvil', level: 4 }]
  }
}
