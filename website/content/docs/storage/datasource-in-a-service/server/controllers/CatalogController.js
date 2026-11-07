@RequestRoute("/catalog")
@Public
class CatalogController {
  /** @type {CatalogService} */
  @Needs
  catalogService;
  /** @returns {Promise<object[]>} */
  @GetRoute
  getCatalog() { return this.catalogService.find(); }
}