use crate::models::UserSaved;

#[Service]
pub struct UserListService {
    #[Needs]
    users_client: std::sync::Arc<UsersClient>,
    pub names: Vec<String>,
}

impl UserListService {
    #[OnEvent]
    async fn user_saved(&mut self, _saved: UserSaved) -> BehaviorResult<()> {
        let users = self.users_client.all().await?;
        self.names = users.into_iter().map(|user| user.name).collect();
        Ok(())
    }
}
