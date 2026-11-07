import type { Product } from '/models/Product.ts'
import type { Summary } from '/models/Summary.ts'

@Service
export default class CatalogService {
  @Needs
  productConverter!: ProductConverter;
  result: string = '';

  async load(): Promise<void> {
    const detail = {name: 'detail', secret: 'detail-private'};
    const product = await this.productConverter.copy({id: 7, name: 'Anvil', detail, items: [detail], secret: 'product-private'});
    const summary = await this.summaryOf(product);
    this.result = this.text(summary);
  }

  private async summaryOf(product: Product | void): Promise<Summary | void> {
    return product ? this.productConverter.map(product) : undefined;
  }

  private text(summary: Summary | void): string {
    return summary ? `${summary.kind} ${summary.id}: ${summary.name}, ${summary.detail.name}, ${summary.items.length}` : '';
  }
}
