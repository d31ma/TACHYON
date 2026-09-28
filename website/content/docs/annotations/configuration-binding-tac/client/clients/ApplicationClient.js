@HTTPClient({"adapter":"publicConfiguration"})
class ApplicationClient {
  /** @returns {Promise<object>} */
  @GetRequest("/manifest.webmanifest")
  async load() { return {}; }
}
