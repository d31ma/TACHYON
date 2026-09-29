@Service
export default class SettingsService {
  @Needs
  readonly applicationClient!: ApplicationClient;
  title = 'Catalog';
  pageSize = 20;
  @OnStart
  async initialize() {
    const application = await this.applicationClient.load();
    const catalog = Behaviors.properties(CatalogSetup, application);
    const values = Behaviors.values(PublicValues, application);
    this.title = String(values.title);
    this.pageSize = Number(catalog.pageSize);
    const lifetime = new AbortController();
    const context = await Behaviors.create(CatalogSetup, {
      partition: 'public-catalog',
      signal: lifetime.signal,
      profiles: ['public'],
      properties: application,
    });
    context.dispose();
    lifetime.abort();
  }
}
