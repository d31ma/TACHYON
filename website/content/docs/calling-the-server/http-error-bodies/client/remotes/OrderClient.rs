use crate::models::OrderInput;

#[HTTPClient("/api")]
trait OrderClient {
    #[PostRequest("/orders")]
    async fn send(&self, #[RequestBody] order: BehaviorValue) -> BehaviorResult<OrderInput>;
}
