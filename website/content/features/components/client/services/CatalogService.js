@Service
export default class CatalogService {
  /** @type {{ id: string, name: string, price: string }[]} */
  products = [
    { id: 'anvil', name: 'Anvil', price: '£42.00' },
    { id: 'rope', name: 'Rope', price: '£8.00' },
  ]
}