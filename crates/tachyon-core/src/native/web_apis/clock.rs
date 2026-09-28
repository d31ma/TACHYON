
#[derive(Clone, Copy, Debug, Default)]
pub struct Clock;

impl Clock {
    pub fn now_ms(&self) -> f64 {
        TacWebHost::epoch_ms()
    }

    pub fn monotonic_ms(&self) -> f64 {
        TacWebHost::monotonic_ms()
    }

    pub fn time_zone(&self) -> Result<String, String> {
        let value = TacWebHost::call("clock", "timeZone", "")?;
        Ok(value.as_text())
    }
}
