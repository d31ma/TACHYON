#[Service]
pub struct GreetingService {
    #[Needs]
    greeting_client: std::sync::Arc<GreetingClient>,
    #[Needs]
    override_client: std::sync::Arc<OverrideClient>,
    pub message: String,
    pub custom: String,
    pub loads: i64,
    pub clears: i64,
}

impl GreetingService {
    pub async fn ask(&mut self) -> BehaviorResult<()> {
        self.loads += 1;
        self.message = self.greeting_client.read().await?.message;
        Ok(())
    }

    pub async fn clear(&mut self) -> BehaviorResult<()> {
        self.greeting_client.clear().await?;
        self.clears += 1;
        Ok(())
    }

    pub async fn ask_custom(&mut self) -> BehaviorResult<()> {
        self.custom = self.override_client.read().await?.message;
        Ok(())
    }
}
