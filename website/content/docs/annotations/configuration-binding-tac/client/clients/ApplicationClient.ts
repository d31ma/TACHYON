@HTTPClient({"adapter":"publicConfiguration"})
class ApplicationClient {
  @GetRequest("/manifest.webmanifest")
  async load(): Promise<BehaviorJsonValue> { return {}; }
}
