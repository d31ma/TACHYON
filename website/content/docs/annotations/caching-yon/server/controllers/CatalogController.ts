import type { CacheReport } from '/models/CacheReport.ts'

@RequestRoute("/catalog")
@Public
class CatalogController {
  @Needs
  readonly catalog!: CatalogService;

  @GetRoute
  async getCatalog(): Promise<CacheReport> {
    const sku = 'anvil';
    const first = await this.catalog.find(sku);
    const repeat = await this.catalog.find(sku);
    await this.catalog.write(sku, 99);
    const written = await this.catalog.find(sku);
    await this.catalog.invalidate(sku);
    const refreshed = await this.catalog.find(sku);
    return { first, repeat, written, refreshed };
  }
}
