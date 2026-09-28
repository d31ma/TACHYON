@Service
export default class CatalogService {
  @Needs
  readonly productsClient!: ProductsClient;
  products: BehaviorJsonValue = [];
  async load() {
    const response = await fetch('/api/products', { credentials: 'omit' });
    this.products = await this.productsFrom(response);
  }
  async save(_event: unknown, product: BehaviorJsonValue) {
    await this.productsClient.save(product);
  }
  private productsFrom(response: Response): Promise<BehaviorJsonValue> {
    if (!response.ok) throw new Error('Products unavailable');
    return response.json();
  }
}