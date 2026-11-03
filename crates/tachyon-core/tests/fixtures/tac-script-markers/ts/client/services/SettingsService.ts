@Service
export default class SettingsService {
  private static readonly ENABLED = 'true';

  catalog(size: number): BehaviorJsonValue {
    return Behaviors.properties(CatalogSetup, SettingsService.published(size, SettingsService.ENABLED));
  }

  values(application: BehaviorJsonObject): BehaviorJsonValue {
    return Behaviors.values(PublicValues, application);
  }

  async open(profiles: string[], enabled: string): Promise<string> {
    const lifetime = new AbortController();
    const properties = SettingsService.published(20, enabled);
    const context = await Behaviors.create(CatalogSetup, { partition: 'catalog', signal: lifetime.signal, profiles, properties });
    context.dispose();
    lifetime.abort();
    return 'open';
  }

  private static published(size: number, enabled: string): BehaviorJsonObject {
    const catalog: BehaviorJsonObject = { title: 'Shop', page: { size } };
    if (enabled) catalog.cache = { enabled: enabled === SettingsService.ENABLED };
    return { tac: { catalog } };
  }
}
