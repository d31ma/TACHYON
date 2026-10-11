[HTTPClient]
public interface ApplicationClient {
    [GetRequest("/manifest.webmanifest")]
    System.Threading.Tasks.Task<BehaviorJSON> Load();
}
