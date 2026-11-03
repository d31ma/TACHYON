use crate::models::PublicSettings;

#[Service]
pub struct SettingsService {
    #[Needs]
    application_client: std::sync::Arc<ApplicationClient>,
    pub api: String,
}

impl SettingsService {
    pub async fn load(&mut self) -> BehaviorResult<()> {
        let application = self.application_client.load().await?;
        let settings = Behaviors::values::<PublicSettings>(&application)?;
        self.api = Self::text_of(settings.get("api"));
        Ok(())
    }

    fn text_of(value: Option<&BehaviorValue>) -> String {
        match value {
            Some(BehaviorValue::String(text)) => text.clone(),
            _ => String::new(),
        }
    }
}
