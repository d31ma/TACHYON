@Service
export default class SettingsService {
  static _ENABLED = 'true';

  /**
   * @param {number} size
   * @returns {BehaviorJsonValue}
   */
  catalog(size) {
    return Behaviors.properties(CatalogSetup, SettingsService._published(size, SettingsService._ENABLED));
  }

  /**
   * @param {BehaviorJsonObject} application
   * @returns {BehaviorJsonValue}
   */
  values(application) {
    return Behaviors.values(PublicValues, application);
  }

  /**
   * @param {string[]} profiles
   * @param {string} enabled
   * @returns {Promise<string>}
   */
  async open(profiles, enabled) {
    const lifetime = new AbortController();
    const properties = SettingsService._published(20, enabled);
    const context = await Behaviors.create(CatalogSetup, { partition: 'catalog', signal: lifetime.signal, profiles, properties });
    context.dispose();
    lifetime.abort();
    return 'open';
  }

  /**
   * @param {number} size
   * @param {string} enabled
   * @returns {BehaviorJsonObject}
   */
  static _published(size, enabled) {
    /** @type {BehaviorJsonObject} */
    const catalog = { title: 'Shop', page: { size } };
    if (enabled) catalog.cache = { enabled: enabled === SettingsService._ENABLED };
    return { tac: { catalog } };
  }
}
