@Service
class CatalogService {
  @Needs
  readonly catalog!: CatalogClient;
  async products(): Promise<BehaviorJsonValue> { return await this.catalog.all(1); }
}
