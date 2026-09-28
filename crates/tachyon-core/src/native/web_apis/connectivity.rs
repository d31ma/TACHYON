
#[derive(Clone, Copy, Debug, Default)]
pub struct Connectivity;

impl Connectivity {
    pub fn online(&self) -> Result<bool, String> {
        let value = TacWebHost::call("connectivity", "online", "")?;
        Ok(value.as_flag())
    }
}
