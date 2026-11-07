#[Service]
pub struct CounterService {
    #[Needs]
    greeting_client: std::sync::Arc<GreetingClient>,
    pub count: i32,
    pub message: String,
}

impl CounterService {
    pub fn increment(&mut self) {
        self.count += 1;
    }

    pub async fn ask_server(&mut self) -> BehaviorResult<()> {
        self.message = self.greeting_client.get_greeting().await?.message;
        Ok(())
    }
}
