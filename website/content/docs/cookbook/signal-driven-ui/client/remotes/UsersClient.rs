use crate::models::UserSaved;

#[HTTPClient]
trait UsersClient {
    #[PostRequest("/users")]
    async fn save(&self, #[RequestBody] name: &str) -> BehaviorResult<UserSaved>;
}
