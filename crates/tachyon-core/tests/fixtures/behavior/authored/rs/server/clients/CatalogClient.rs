#[HTTPClient({"setup": "ConfigSetup"})]
trait CatalogClient {
    #[GetRequest("/products")]
    fn remote(&self) -> BehaviorResult<BehaviorValue>;

    #[GetRequest("/products")]
    fn search(&self, #[RequestParam("page")] page: BehaviorValue) -> BehaviorResult<BehaviorValue>;

    #[GetRequest("/products/{id}")]
    fn find(&self, #[PathVariable] id: BehaviorValue) -> BehaviorResult<BehaviorValue>;
}
