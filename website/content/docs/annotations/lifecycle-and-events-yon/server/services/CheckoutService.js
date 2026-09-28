@Service
class CheckoutService {
  /** @type {ApplicationEventPublisher} */
  @Needs
  applicationEventPublisher;

  @OnStart
  start() {
    /** @type {PlacedOrder} */
    const order = { status: 'paid', total: 25 };
    this.applicationEventPublisher.publishEvent(order);
  }

  @OnStop
  stop() {}
}
