@Service
export default class CatalogService {
  @Needs
  readonly catalogClient!: CatalogClient;
  count = 0;

  async load(): Promise<void> {
    const answer = await this.catalogClient.all();
    this.count = this.countItems(answer);
  }

  private countItems(answer: { items: string[] } | void): number {
    return answer ? answer.items.length : 0;
  }
}
