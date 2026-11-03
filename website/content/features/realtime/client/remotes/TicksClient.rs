use crate::models::Tick;

#[HTTPClient("/ticks")]
trait TicksClient {
    #[StreamRequest]
    fn stream_ticks(&self, tick: Tick);
}
