import type { Product } from '/models/Product.ts'

@Service
export default class CatalogService {
  @Needs
  productConverter!: ProductConverter;
  result: string = '';

  async load(): Promise<void> {
    const detail = {name: 'detail', secret: 'hidden'};
    const product = await this.productConverter.copy({id: 7, name: 'Anvil', detail, items: [detail], secret: 'hidden'});
    this.result = await this.summaryText(product);
  }

  private async summaryText(product: Product | void): Promise<string> {
    return product ? JSON.stringify(await this.productConverter.map(product)) : '';
  }
}
