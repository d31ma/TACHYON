[HTTPClient]
interface CatalogClient {
    [GetRequest("/api/products")]
    System.Threading.Tasks.Task<BehaviorJSON> All();
}
