@RequestRoute("/catalog")
@Public
class CatalogController {
@Needs
productConverter;
/** @returns {Summary} */
@GetRoute
async getCatalog() {
const detail = {name: 'detail', secret: 'hidden'};
return this.productConverter.map({id: 7, name: 'Anvil', detail, items: [detail], secret: 'hidden'});
}
}
