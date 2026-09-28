@Service
export default class CatalogService {
  @Needs
  readonly catalogClient!: CatalogClient;
  count = 0;

  async load(): Promise<void> {
    const answer = await this.catalogClient.all();
    this.count = this.countItems(answer);
  }

  private countItems(answer: unknown): number {
    const items = (answer as { items: unknown }).items;
    return Array.isArray(items) ? items.length : 0;
  }
}
