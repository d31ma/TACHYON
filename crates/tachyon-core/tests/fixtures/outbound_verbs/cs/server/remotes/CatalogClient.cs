[HTTPClient("/api")]
public interface CatalogClient {
    [GetRequest("/products/{id}")]
    System.Threading.Tasks.Task<BehaviorJSON> Find([PathVariable("id")] string id, [RequestParam("view")] string view, [RequestHeader("X-Tenant")] string tenant);

    [PostRequest("/products")]
    System.Threading.Tasks.Task<BehaviorJSON> Save([RequestBody] BehaviorJSON product);

    [PutRequest("/products/{id}")]
    System.Threading.Tasks.Task<BehaviorJSON> Replace([PathVariable("id")] long id, [RequestBody] BehaviorJSON product);

    [PatchRequest("/products/{id}")]
    System.Threading.Tasks.Task<BehaviorJSON> Rename([PathVariable("id")] long id, [RequestBody] BehaviorJSON product);

    [DeleteRequest("/products/{id}")]
    System.Threading.Tasks.Task<BehaviorJSON> Remove([PathVariable("id")] long id);
}
