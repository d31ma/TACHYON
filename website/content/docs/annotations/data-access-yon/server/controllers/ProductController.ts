import type { ProductReport } from '/models/ProductReport.ts'

@RequestRoute("/products")
@Public
class ProductController {
  @GetRoute
  async getProduct(): Promise<ProductReport> {
    const context = await Behaviors.create(CatalogSetup, {
      partition: 'public-catalog',
      signal: AbortSignal.timeout(5000),
    });
    const products = Behaviors.construct(ProductStore, { behaviors: context });
    const activeStatus = 'active';
    const active = await products.findByStatus(activeStatus);
    const activeCount = await products.countByStatus(activeStatus);
    const page = products.bounded({ size: 5 });
    const others = await products.excluding(activeStatus, page);
    context.dispose();
    return { active, count: activeCount, others };
  }
}
