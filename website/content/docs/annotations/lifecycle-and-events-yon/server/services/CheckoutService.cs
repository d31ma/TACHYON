[Service]
class CheckoutService {
  [Needs]
  readonly ApplicationEventPublisher _applicationEventPublisher;

  [OnStart]
  public void Start() {
    _applicationEventPublisher.PublishEvent(new Models.PlacedOrder("paid", 25));
  }

  [OnStop]
  public void Stop() {}
}
