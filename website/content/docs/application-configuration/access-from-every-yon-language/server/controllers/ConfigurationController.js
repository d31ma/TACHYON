/** @import { Configuration } from '/models/Configuration.ts' */
@RequestRoute("/configuration")
@Public
class ConfigurationController {
  /** @returns {Configuration} */
  @GetRoute
  getConfiguration() {
    const application = JSON.parse(process.env.YON_APPLICATION_JSON);
    const settings = Behaviors.properties(CatalogSettings, application);
    const endpoint = Behaviors.values(Endpoint, application);
    return { pageSize: settings.pageSize, port: endpoint.port };
  }
}
