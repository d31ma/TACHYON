import type { Product } from '/models/Product.ts'

@Service
export default class CatalogService {
  products: Product[] = [
    { id: 'anvil', name: 'Anvil', price: '£42.00' },
    { id: 'rope', name: 'Rope', price: '£8.00' },
  ]
}
