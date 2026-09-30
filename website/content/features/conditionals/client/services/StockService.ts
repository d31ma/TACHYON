import type { Product } from '/models/Product.ts'

@Service
export default class StockService {
  products: Product[] = []
  restocking: boolean = true
}
