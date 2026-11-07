using Models;

[Service]
public class CheckoutService {
    public int Placed = 0;
    private readonly ApplicationEventPublisher applicationEventPublisher;

    [OnStart]
    public void Start() { Place(); }

    public void Place() {
        Placed += 1;
        applicationEventPublisher.PublishEvent(new PlacedOrder("paid", 25), true);
    }

    [OnStop]
    public void Stop() { Placed = 0; }
}
