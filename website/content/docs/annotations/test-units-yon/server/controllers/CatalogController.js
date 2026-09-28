/** @import { CatalogTotal } from '/models/CatalogTotal.ts' */
@RequestRoute("/api/catalog")
@Public
class CatalogController {
  /** @type {CatalogService} */
  @Needs
  catalogService;

  /** @returns {CatalogTotal} */
  @GetRoute
  getTotal() {
    return { total: this.catalogService.total() }
  }
}
