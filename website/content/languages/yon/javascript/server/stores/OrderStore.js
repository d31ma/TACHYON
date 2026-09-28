@Store
export class OrderStore {
  /** @returns {Order[]} */
  all() {
    return [{ sku: 'anvil', level: 4 }]
  }
}
