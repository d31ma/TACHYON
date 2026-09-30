@Service
export default class CatalogService {
  @Needs
  converter!: ProductConverter;
  title = 'Not mapped.';

  async load() {
    const product = { detail: { id: 7 }, name: 'Anvil', extra: 'dropped' };
    const summary = await this.converter.toSummary(product);
    this.title = this.titleOf(summary);
  }

  private titleOf(summary: BehaviorJsonObject | void): string {
    return summary ? `${summary.kind} ${summary.productId}: ${summary.title}` : '';
  }
}
