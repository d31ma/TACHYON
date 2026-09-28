@Service
export default class ProductService {
  summary = 'Not loaded.';

  async load() {
    const context = await Behaviors.create(CatalogSetup, {
      partition: 'public-catalog',
      signal: AbortSignal.timeout(5000),
    });
    const products = Behaviors.construct(ProductStore, { behaviors: context });
    const page = products.bounded({ size: 5 });
    const rows = await products.excluding('retired', page);
    const activeCount = await products.countByStatus('active');
    this.summary = `${rows.length} of ${activeCount} active`;
    context.dispose();
  }
}
