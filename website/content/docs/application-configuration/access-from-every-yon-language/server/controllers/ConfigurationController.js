/** @import { Configuration } from '/models/Configuration.ts' */
@RequestRoute("/configuration")
@Public
class ConfigurationController {
  /** @returns {Configuration} */
  @GetRoute
  getConfiguration() {
    const application = JSON.parse(String(process.env.YON_APPLICATION_JSON));
    const settings = Behaviors.properties(CatalogSettings, application);
    const endpoint = Behaviors.values(Endpoint, application);
    return { pageSize: Number(settings.pageSize), port: Number(endpoint.port) };
  }
}
