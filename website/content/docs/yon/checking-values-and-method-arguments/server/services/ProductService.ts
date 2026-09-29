import type { Product } from '/models/Product.ts';

@Service
export default class ProductService {
  accept(@Check input: ProductInput): Product {
    return input as Product;
  }
}
