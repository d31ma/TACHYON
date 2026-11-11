
#[derive(Clone, Copy, Debug, Default)]
pub struct Push;

impl Push {
    const API: &'static str = "push";

    pub fn supported(&self) -> Result<bool, String> {
        let value = TacWebHost::call(Self::API, "supported", "")?;
        Ok(value.as_flag())
    }

    pub fn subscribe(&self, application_server_key: &str) -> Result<(), String> {
        TacWebHost::call(Self::API, "subscribe", &tac_quote(application_server_key))?;
        Ok(())
    }

    pub fn subscription(&self) -> Result<(), String> {
        TacWebHost::call(Self::API, "subscription", "")?;
        Ok(())
    }

    pub fn unsubscribe(&self) -> Result<(), String> {
        TacWebHost::call(Self::API, "unsubscribe", "")?;
        Ok(())
    }
}
