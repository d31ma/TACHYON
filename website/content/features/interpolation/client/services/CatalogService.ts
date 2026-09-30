import type { Product } from '/models/Product.ts'

@Service
export default class CatalogService {
  title: string = 'Catalog'
  count: number = 6
  product: Product = { name: 'Anvil', price: '£42.00' }
}
