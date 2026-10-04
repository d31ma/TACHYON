use crate::models::Greeting;

#[HTTPClient("/api")]
trait OverrideClient {
    #[GetRequest("/never")]
    async fn read(&self) -> BehaviorResult<Greeting>;
}
