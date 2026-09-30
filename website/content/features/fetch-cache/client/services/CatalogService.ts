@Service
export default class CatalogService {
  @Needs
  productsClient!: ProductsClient;
  count = 0;

  async load(): Promise<void> {
    this.count = this.countItems(await this.productsClient.list());
  }

  async add(): Promise<void> {
    await this.productsClient.save({ name: 'anvil' });
  }

  private countItems(answer: { items: string[] } | void): number {
    return answer ? answer.items.length : 0;
  }
}
