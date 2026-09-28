@Service
export default class CheckoutService {
  /** @type {HTMLElement} */
  @Needs
  view;
  /** @type {AbortSignal} */
  @Needs
  lifecycle;
  /** @type {ApplicationEventPublisher} */
  applicationEventPublisher;
  placed = 0;

  @OnStart
  start() {
    this.view.dataset.ready = 'true';
    this.place();
  }

  place() {
    this.placed += 1;
    /** @type {PlacedOrder} */
    const order = { status: 'paid', total: 25 };
    this.applicationEventPublisher.publishEvent(order, true);
  }

  @OnStop
  stop() { this.placed = 0; }
}
