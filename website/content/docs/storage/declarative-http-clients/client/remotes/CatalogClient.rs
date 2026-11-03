#[HTTPClient]
trait CatalogClient {
    #[GetRequest("/api/products")]
    fn all(&self) -> BehaviorResult<BehaviorValue>;
}
