@Service
export class OrderService {
  /** @returns {Order[]} */
  list() {
    return [{ sku: 'anvil', level: 4 }]
  }
}
