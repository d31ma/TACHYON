@RequestRoute("/catalog")
@Public
class CatalogController {
  @Needs
  readonly catalog!: CatalogService;
  @GetRoute
  async getCatalog() { return await this.catalog.products(); }
}
