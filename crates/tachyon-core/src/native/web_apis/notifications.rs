
#[derive(Clone, Copy, Debug, Default)]
pub struct Notifications;

impl Notifications {
    const API: &'static str = "notifications";

    pub fn permission(&self) -> Result<String, String> {
        let value = TacWebHost::call(Self::API, "permission", "")?;
        Ok(value.as_text())
    }

    pub fn request_permission(&self) -> Result<(), String> {
        TacWebHost::call(Self::API, "requestPermission", "")?;
        Ok(())
    }

    pub fn show(&self, title: &str, body: &str) -> Result<(), String> {
        let arguments = [tac_quote(title), tac_quote(body)].join(",");
        TacWebHost::call(Self::API, "show", &arguments)?;
        Ok(())
    }
}
