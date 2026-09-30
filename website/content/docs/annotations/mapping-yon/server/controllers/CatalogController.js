/** @import { CatalogSummary } from '/models/CatalogSummary.ts' */
@RequestRoute("/catalog")
@Public
class CatalogController {
  /** @type {ProductConverter} */
  @Needs
  productConverter;

  /** @returns {Promise<CatalogSummary>} */
  @GetRoute
  async getCatalog() {
    const product = { detail: { id: 7 }, name: 'Anvil', extra: 'dropped' };
    const summary = await this.productConverter.toSummary(product);
    return { summary };
  }
}
