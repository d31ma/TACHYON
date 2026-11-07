import models.Tick;

@HTTPClient("/ticks")
interface TicksClient {
    @StreamRequest
    void receive(Tick tick);
}
