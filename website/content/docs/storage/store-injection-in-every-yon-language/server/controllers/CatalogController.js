@RequestRoute("/catalog")
@Public
class CatalogController {
  /** @type {CatalogStore} */
  @Needs
  catalogStore;
  /** @returns {Promise<object[]>} */
  @GetRoute
  getCatalog() { return this.catalogStore.find(); }
}