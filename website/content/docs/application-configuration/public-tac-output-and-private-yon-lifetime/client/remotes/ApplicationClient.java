@HTTPClient({"adapter":"publicConfiguration"})
interface ApplicationClient {
    @GetRequest("/manifest.webmanifest")
    Object load();
}
