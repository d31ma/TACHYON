/** @import { Settings } from '/models/Settings.ts' */
@RequestRoute("/settings")
@Public
class SettingsController {
  /** @returns {Promise<Settings>} */
  @GetRoute
  async getSettings() {
    const application = JSON.parse(String(process.env.YON_APPLICATION_JSON));
    const catalog = Behaviors.properties(CatalogSetup, application);
    const endpoint = Behaviors.values(Endpoint, application);
    const lifetime = new AbortController();
    const context = await Behaviors.create(CatalogSetup, {
      partition: 'public-catalog',
      signal: lifetime.signal,
      profiles: [String(endpoint.profile)],
      properties: application,
    });
    context.dispose();
    lifetime.abort();
    return { profile: String(endpoint.profile), pageSize: Number(catalog.pageSize) };
  }
}
