#[Service]
pub struct StreamService {
    #[Needs]
    ticks_client: std::sync::Arc<TicksClient>,
    pub ticks: Vec<Tick>,
}

impl StreamService {
    const TICK_LIMIT: usize = 128;

    #[OnEvent]
    fn add_tick(&mut self, tick: Tick) {
        self.ticks.push(tick);
        let extra = self.ticks.len().saturating_sub(Self::TICK_LIMIT);
        self.ticks.drain(..extra);
    }
}
