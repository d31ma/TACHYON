@Service
export default class CatalogService {
  @Needs
  readonly catalog!: CatalogClient;
  async products(): Promise<BehaviorJsonValue | void> { return await this.catalog.all(1); }
}
