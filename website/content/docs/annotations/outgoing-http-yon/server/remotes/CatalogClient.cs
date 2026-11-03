[HTTPClient]
public interface CatalogClient {
    [GetRequest("/products")]
    System.Threading.Tasks.Task<BehaviorJSON> All([RequestParam("page")] BehaviorJSON page);

    [PostRequest("/products")]
    System.Threading.Tasks.Task<BehaviorJSON> Add([RequestBody] BehaviorJSON product);
}
