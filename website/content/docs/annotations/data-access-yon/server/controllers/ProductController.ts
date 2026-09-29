import type { ProductReport } from '/models/ProductReport.ts'

@RequestRoute("/products")
@Public
class ProductController {
  @Needs
  products!: ProductStore;

  @GetRoute
  async getProduct(): Promise<ProductReport> {
    const activeStatus = 'active';
    const active = await this.products.findByStatus(activeStatus);
    const activeCount = await this.products.countByStatus(activeStatus);
    const others = await this.products.excluding(activeStatus, { size: 5 });
    return { active, count: activeCount, others };
  }
}
