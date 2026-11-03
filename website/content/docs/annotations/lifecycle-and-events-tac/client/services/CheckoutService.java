import models.PlacedOrder;

@Service
class CheckoutService {
    int placed = 0;
    ApplicationEventPublisher applicationEventPublisher;

    @OnStart
    void start() { place(); }

    void place() {
        placed += 1;
        applicationEventPublisher.publishEvent(new PlacedOrder("paid", 25), true);
    }

    @OnStop
    void stop() { placed = 0; }
}
