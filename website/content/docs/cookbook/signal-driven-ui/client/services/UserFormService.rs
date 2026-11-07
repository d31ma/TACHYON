#[Service]
pub struct UserFormService {
    #[Needs]
    users_client: std::sync::Arc<UsersClient>,
    application_event_publisher: ApplicationEventPublisher,
    pub name: String,
}

impl UserFormService {
    pub async fn save(&mut self) -> BehaviorResult<()> {
        let saved = self.users_client.save(&self.name).await?;
        self.application_event_publisher.publish_event(saved, false);
        Ok(())
    }
}
