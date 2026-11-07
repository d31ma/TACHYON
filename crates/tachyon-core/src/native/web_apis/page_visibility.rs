
#[derive(Clone, Copy, Debug, Default)]
pub struct PageVisibility;

impl PageVisibility {
    pub fn visible(&self) -> Result<bool, String> {
        let value = TacWebHost::call("pageVisibility", "visible", "")?;
        Ok(value.as_flag())
    }
}
