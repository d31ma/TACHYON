#[HTTPClient("/api")]
trait CatalogClient {
    #[GetRequest("/products/{id}")]
    fn find(&self, #[PathVariable("id")] id: BehaviorValue, #[RequestParam("view")] view: BehaviorValue, #[RequestHeader("X-Tenant")] tenant: BehaviorValue) -> BehaviorResult<BehaviorValue>;

    #[PostRequest("/products")]
    fn save(&self, #[RequestBody] product: BehaviorValue) -> BehaviorResult<BehaviorValue>;

    #[PutRequest("/products/{id}")]
    fn replace(&self, #[PathVariable("id")] id: BehaviorValue, #[RequestBody] product: BehaviorValue) -> BehaviorResult<BehaviorValue>;

    #[PatchRequest("/products/{id}")]
    fn rename(&self, #[PathVariable("id")] id: BehaviorValue, #[RequestBody] product: BehaviorValue) -> BehaviorResult<BehaviorValue>;

    #[DeleteRequest("/products/{id}")]
    fn remove(&self, #[PathVariable("id")] id: BehaviorValue) -> BehaviorResult<BehaviorValue>;
}
