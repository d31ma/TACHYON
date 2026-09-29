@HTTPClient({"adapter":"publicConfiguration"})
export default class ApplicationClient {
  /** @returns {Promise<BehaviorJsonObject>} */
  @GetRequest("/manifest.webmanifest")
  async load() {}
}
