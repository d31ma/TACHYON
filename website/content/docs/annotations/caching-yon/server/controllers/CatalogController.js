/** @import { CacheReport } from '/models/CacheReport.ts' */
@RequestRoute("/catalog")
@Public
class CatalogController {
  /** @type {CatalogService} */
  @Needs
  catalogService;

  /** @returns {Promise<CacheReport>} */
  @GetRoute
  async getCatalog() {
    const sku = 'anvil';
    const first = await this.catalogService.find(sku);
    const repeat = await this.catalogService.find(sku);
    await this.catalogService.write(sku, 99);
    const written = await this.catalogService.find(sku);
    await this.catalogService.invalidate(sku);
    const refreshed = await this.catalogService.find(sku);
    return { first, repeat, written, refreshed };
  }
}
