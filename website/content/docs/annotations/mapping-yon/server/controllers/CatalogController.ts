import type { CatalogSummary } from '/models/CatalogSummary.ts'

@RequestRoute("/catalog")
@Public
class CatalogController {
  @Needs
  productConverter!: ProductConverter;

  @GetRoute
  async getCatalog(): Promise<CatalogSummary> {
    const product = { detail: { id: 7 }, name: 'Anvil', extra: 'dropped' };
    const summary = await this.productConverter.toSummary(product);
    return { summary };
  }
}
