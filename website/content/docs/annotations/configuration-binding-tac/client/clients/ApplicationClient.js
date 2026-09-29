@HTTPClient({"adapter":"publicConfiguration"})
export default class ApplicationClient {
  /** @returns {Promise<BehaviorJsonObject | void>} */
  @GetRequest("/manifest.webmanifest")
  async load() {}
}
