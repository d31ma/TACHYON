#[derive(Clone, Copy, Debug, Default)]
pub struct Locks;

impl Locks {
    const API: &'static str = "locks";

    pub fn request(&self, name: &str) -> Result<(), String> {
        TacWebHost::call(Self::API, "request", &tac_quote(name))?;
        Ok(())
    }

    pub fn release(&self, name: &str) -> Result<(), String> {
        TacWebHost::call(Self::API, "release", &tac_quote(name))?;
        Ok(())
    }
}
