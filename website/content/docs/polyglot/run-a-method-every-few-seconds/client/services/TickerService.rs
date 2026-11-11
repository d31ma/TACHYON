#[Service]
#[derive(Default)]
pub struct TickerService {
    pub seconds: i64,
}

impl TickerService {
    #[Every("1s")]
    pub fn count(&mut self) {
        self.seconds += 1;
    }
}
