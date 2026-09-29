@RequestRoute("/catalog")
@Public
class CatalogController {
  @Needs
  catalogStore!: CatalogStore;
  @GetRoute
  getCatalog() { return this.catalogStore.find(); }
}