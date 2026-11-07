@RequestRoute("/catalog")
@Public
class CatalogController {
  @Needs
  catalogService!: CatalogService;
  @GetRoute
  getCatalog() { return this.catalogService.find(); }
}