@Service
export default class CatalogService {
  /** @type {ProductConverter} */
  @Needs
  productConverter;
  result = '';
  async load() {
    const detail = {name: 'detail', secret: 'hidden'};
    const product = await this.productConverter.copy({id: 7, name: 'Anvil', detail, items: [detail], secret: 'hidden'});
    this.result = JSON.stringify(await this.productConverter.map(product));
  }
}
