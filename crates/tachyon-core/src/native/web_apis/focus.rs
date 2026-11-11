
#[derive(Clone, Copy, Debug, Default)]
pub struct Focus;

impl Focus {
    pub fn move_to(&self, target: &str) -> Result<(), String> {
        TacWebHost::call("focus", "move", &tac_quote(target))?;
        Ok(())
    }

    pub fn trap(&self, container: &str) -> Result<(), String> {
        TacWebHost::call("focus", "trap", &tac_quote(container))?;
        Ok(())
    }

    pub fn release(&self) -> Result<(), String> {
        TacWebHost::call("focus", "release", "")?;
        Ok(())
    }
}
