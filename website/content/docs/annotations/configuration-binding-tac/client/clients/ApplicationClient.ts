@HTTPClient({"adapter":"publicConfiguration"})
export default class ApplicationClient {
  @GetRequest("/manifest.webmanifest")
  async load(): Promise<BehaviorJsonObject | void> {}
}
