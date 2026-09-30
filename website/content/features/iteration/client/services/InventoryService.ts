import type { Product } from '/models/Product.ts'

@Service
export default class InventoryService {
  products: Product[] = [{ name: 'Anvil' }, { name: 'Rope' }]
}
