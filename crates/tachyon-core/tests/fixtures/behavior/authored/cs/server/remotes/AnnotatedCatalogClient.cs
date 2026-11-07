[HTTPClient({"adapter": "catalog", "setup": "AnnotatedConfigSetup"})]
public interface AnnotatedCatalogClient {
    [GetRequest("/products")]
    System.Threading.Tasks.Task<BehaviorJSON> remote();
    [GetRequest("/products")]
    System.Threading.Tasks.Task<BehaviorJSON> search([RequestParam("page")] BehaviorJSON page);
    [GetRequest("/products/{id}")]
    System.Threading.Tasks.Task<BehaviorJSON> find([PathVariable] BehaviorJSON id);
}
