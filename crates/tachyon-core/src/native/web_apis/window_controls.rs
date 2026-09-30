
#[derive(Clone, Copy, Debug, Default)]
pub struct WindowControls;

impl WindowControls {
    const API: &'static str = "windowControls";

    pub fn granted(&self, control: &str) -> Result<bool, String> {
        let value = TacWebHost::call(Self::API, "granted", &tac_quote(control))?;
        Ok(value.as_flag())
    }

    pub fn minimize(&self) -> Result<(), String> {
        TacWebHost::call(Self::API, "minimize", "")?;
        Ok(())
    }

    pub fn maximize(&self) -> Result<(), String> {
        TacWebHost::call(Self::API, "maximize", "")?;
        Ok(())
    }

    pub fn fullscreen(&self) -> Result<(), String> {
        TacWebHost::call(Self::API, "fullscreen", "")?;
        Ok(())
    }

    pub fn close(&self) -> Result<(), String> {
        TacWebHost::call(Self::API, "close", "")?;
        Ok(())
    }

    pub fn resize(&self, width: u32, height: u32) -> Result<(), String> {
        TacWebHost::call(Self::API, "resize", &format!("{width},{height}"))?;
        Ok(())
    }

    pub fn set_title(&self, text: &str) -> Result<(), String> {
        TacWebHost::call(Self::API, "setTitle", &tac_quote(text))?;
        Ok(())
    }
}
