
#[derive(Clone, Copy, Debug, Default)]
pub struct CodeScanner;

impl CodeScanner {
    const API: &'static str = "codeScanner";
    const NO_ARGUMENTS: &'static str = "";

    pub fn supported(&self) -> Result<bool, String> {
        Ok(TacWebHost::call(Self::API, "supported", Self::NO_ARGUMENTS)?.as_flag())
    }

    pub fn scan(&self) -> Result<(), String> {
        Self::send("scan")
    }

    pub fn cancel(&self) -> Result<(), String> {
        Self::send("cancel")
    }

    fn send(call: &str) -> Result<(), String> {
        TacWebHost::call(Self::API, call, Self::NO_ARGUMENTS)?;
        Ok(())
    }
}
