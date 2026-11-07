import models.PlacedOrder;

@Service
class CheckoutService {
  @Needs
  ApplicationEventPublisher applicationEventPublisher;

  @OnStart
  void start() {
    applicationEventPublisher.publishEvent(new PlacedOrder("paid", 25));
  }

  @OnStop
  void stop() {}
}
