use crate::models::UserSaved;

#[HTTPClient({"adapter":"users","setup":"UsersSetup"})]
trait UsersClient {
    #[PostRequest("/users")]
    async fn save(&self, #[RequestBody] name: &str) -> BehaviorResult<UserSaved>;

    #[GetRequest("/users")]
    async fn all(&self) -> BehaviorResult<Vec<UserSaved>>;
}
