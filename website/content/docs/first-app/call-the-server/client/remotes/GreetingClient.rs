use crate::models::Greeting;

#[HTTPClient("/api")]
trait GreetingClient {
    #[GetRequest("/greeting")]
    async fn get_greeting(&self) -> BehaviorResult<Greeting>;
}
