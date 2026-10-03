[HTTPClient({"adapter": "catalog", "setup": "AnnotatedConfigSetup"})]
public interface AnnotatedValidatedCacheClient {
    [GetRequest("/products")]
    System.Threading.Tasks.Task<BehaviorJSON> remote([RequestParam("input")] [Check(typeof(AnnotatedInput))] BehaviorJSON input);
}
