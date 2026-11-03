@Service
export default class ProductService {
  @Needs
  products!: ProductStore;
  summary: string = 'Not loaded.';

  async load(): Promise<void> {
    const rows = await this.products.excluding('retired', { size: 5 });
    const activeCount = await this.products.countByStatus('active');
    this.summary = this.summaryOf(rows, activeCount);
  }

  private summaryOf(rows: ProductEntity[] | void, activeCount: number | void): string {
    return `${rows ? rows.length : 0} of ${activeCount} active`;
  }
}
