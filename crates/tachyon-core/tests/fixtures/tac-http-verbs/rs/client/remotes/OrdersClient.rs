use crate::models::SentRequest;

#[HTTPClient({"path":"/api","adapter":"orders","setup":"OrdersSetup"})]
trait OrdersClient {
    #[PutRequest("/orders/{id}")]
    async fn replace(&self, #[PathVariable("id")] id: String, #[RequestBody] order: BehaviorValue, #[RequestHeader("X-Tenant")] tenant: String) -> BehaviorResult<SentRequest>;

    #[PatchRequest("/orders/{id}")]
    async fn change(&self, #[PathVariable("id")] id: String, #[RequestBody] change: BehaviorValue, #[RequestHeader("X-Tenant")] tenant: String) -> BehaviorResult<SentRequest>;

    #[DeleteRequest("/orders/{id}")]
    async fn remove(&self, #[PathVariable("id")] id: String, #[RequestHeader("X-Reason")] reason: String) -> BehaviorResult<SentRequest>;
}
