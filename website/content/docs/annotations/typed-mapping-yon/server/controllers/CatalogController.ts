import type { Summary } from '/models/Summary.ts'
@RequestRoute("/catalog")
@Public
class CatalogController {
@Needs
converter!: ProductConverter;
@GetRoute
async getCatalog(): Promise<Summary> {
const detail = {name: 'detail', secret: 'detail-private'};
return this.converter.map({id: 7, name: 'Anvil', detail, items: [detail], secret: 'product-private'});
}
}
