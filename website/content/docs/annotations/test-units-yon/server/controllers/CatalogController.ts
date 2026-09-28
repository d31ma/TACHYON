import type { CatalogTotal } from '/models/CatalogTotal.ts'

@RequestRoute("/api/catalog")
@Public
class CatalogController {
  @Needs
  catalogService!: CatalogService;

  @GetRoute
  getTotal(): CatalogTotal {
    return { total: this.catalogService.total() }
  }
}
