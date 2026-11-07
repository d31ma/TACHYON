@RequestRoute("/catalog")
@Public
class CatalogController {
  /** @type {ProductConverter} */
  @Needs
  productConverter;

  /** @returns {Promise<Summary | void>} */
  @GetRoute
  async getCatalog() {
    const detail = {name: 'detail', secret: 'detail-private'};
    return this.productConverter.map({id: 7, name: 'Anvil', detail, items: [detail], secret: 'product-private'});
  }
}
