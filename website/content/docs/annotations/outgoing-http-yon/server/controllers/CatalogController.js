@RequestRoute("/catalog")
@Public
class CatalogController {
  /** @type {CatalogService} */
  @Needs
  catalogService;
  /** @returns {Promise<unknown>} */
  @GetRoute
  async getCatalog() { return await this.catalogService.products(); }
}
