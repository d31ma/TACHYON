
#[derive(Clone, Copy, Debug, Default)]
pub struct Clipboard;

impl Clipboard {
    const API: &'static str = "clipboard";

    pub fn write_text(&self, text: &str) -> Result<(), String> {
        TacWebHost::call(Self::API, "writeText", &tac_quote(text))?;
        Ok(())
    }

    pub fn read_text(&self) -> Result<(), String> {
        TacWebHost::call(Self::API, "readText", "")?;
        Ok(())
    }
}
