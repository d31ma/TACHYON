
#[derive(Clone, Copy, Debug, Default)]
pub struct Passkeys;

impl Passkeys {
    const API: &'static str = "passkeys";
    const NO_ARGUMENTS: &'static str = "";

    pub fn supported(&self) -> Result<bool, String> {
        Ok(TacWebHost::call(Self::API, "supported", Self::NO_ARGUMENTS)?.as_flag())
    }

    pub fn check(&self) -> Result<(), String> {
        Self::send("check", Self::NO_ARGUMENTS)
    }

    pub fn create(&self, options_json: &str) -> Result<(), String> {
        Self::send("create", &tac_quote(options_json))
    }

    pub fn get(&self, options_json: &str) -> Result<(), String> {
        Self::send("get", &tac_quote(options_json))
    }

    pub fn offer(&self, options_json: &str) -> Result<(), String> {
        Self::send("offer", &tac_quote(options_json))
    }

    pub fn cancel(&self) -> Result<(), String> {
        Self::send("cancel", Self::NO_ARGUMENTS)
    }

    fn send(call: &str, arguments: &str) -> Result<(), String> {
        TacWebHost::call(Self::API, call, arguments)?;
        Ok(())
    }
}
