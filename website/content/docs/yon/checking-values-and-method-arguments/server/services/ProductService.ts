import type { Product } from '/models/Product.ts';

@Service
class ProductService {
  accept(@Check input: ProductInput): Product {
    return input;
  }
}
