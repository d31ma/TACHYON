
#[derive(Clone, Copy, Debug, Default)]
pub struct Preferences;

impl Preferences {
    const API: &'static str = "preferences";

    pub fn scheme(&self) -> Result<String, String> {
        Self::text("scheme")
    }

    pub fn motion(&self) -> Result<String, String> {
        Self::text("motion")
    }

    pub fn language(&self) -> Result<String, String> {
        Self::text("language")
    }

    fn text(call: &str) -> Result<String, String> {
        let value = TacWebHost::call(Self::API, call, "")?;
        Ok(value.as_text())
    }
}
