import type { Product } from '/models/Product.ts'
import type { Summary } from '/models/Summary.ts'

@Converter
export default class ProductConverter {
  @Convert({"fields":[{"target":"kind","constant":"product"}]})
  async map(product: Product): Promise<Summary | void> {}
}
