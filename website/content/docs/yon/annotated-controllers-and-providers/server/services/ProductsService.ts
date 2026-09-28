import type { Product } from '/models/Product.ts'

@Service
@Lifetime('transient')
class ProductsService {
  list(): Product[] { return [this.find(1)]; }
  find(id: number): Product { return { id, name: 'Anvil' }; }
}
