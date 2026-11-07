
#[derive(Clone, Copy, Debug, Default)]
pub struct Geolocation;

impl Geolocation {
    pub fn current_position(&self) -> Result<(), String> {
        TacWebHost::call("geolocation", "currentPosition", "")?;
        Ok(())
    }
}
