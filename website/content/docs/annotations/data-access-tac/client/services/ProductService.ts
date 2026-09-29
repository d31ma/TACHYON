@Service
export default class ProductService {
  summary: string = 'Not loaded.';

  async load(): Promise<void> {
    const context = await Behaviors.create(CatalogSetup, {
      partition: 'public-catalog',
      signal: AbortSignal.timeout(5000),
    });
    const products = Behaviors.construct(ProductStore, { behaviors: context });
    const page = products.bounded({ size: 5 });
    const rows = await products.excluding('retired', page);
    const activeCount = await products.countByStatus('active');
    this.summary = this.summaryOf(rows, activeCount);
    context.dispose();
  }

  private summaryOf(rows: ProductEntity[] | void, activeCount: number | void): string {
    return `${rows ? rows.length : 0} of ${activeCount} active`;
  }
}
