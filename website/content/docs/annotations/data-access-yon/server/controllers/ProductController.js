/** @import { ProductReport } from '/models/ProductReport.ts' */
@RequestRoute("/products")
@Public
class ProductController {
  /** @type {ProductStore} */
  @Needs
  productStore;

  /** @returns {Promise<ProductReport>} */
  @GetRoute
  async getProduct() {
    const activeStatus = 'active';
    const active = await this.productStore.findByStatus(activeStatus);
    const activeCount = await this.productStore.countByStatus(activeStatus);
    const others = await this.productStore.excluding(activeStatus, { size: 5 });
    const first = await this.productStore.findById(1);
    return { active, count: activeCount, others, first };
  }
}
