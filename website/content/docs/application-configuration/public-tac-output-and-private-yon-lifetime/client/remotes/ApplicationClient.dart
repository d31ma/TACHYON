@HTTPClient
abstract interface class ApplicationClient {
  @GetRequest("/manifest.webmanifest")
  Future<Object?> load();
}
