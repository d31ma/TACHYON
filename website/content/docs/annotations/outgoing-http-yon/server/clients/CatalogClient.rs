#[HTTPClient]
trait CatalogClient {
    #[GetRequest("/products")]
    fn all(&self, #[RequestParam("page")] page: BehaviorValue) -> BehaviorResult<BehaviorValue>;

    #[PostRequest("/products")]
    fn add(&self, #[RequestBody] product: BehaviorValue) -> BehaviorResult<BehaviorValue>;
}
