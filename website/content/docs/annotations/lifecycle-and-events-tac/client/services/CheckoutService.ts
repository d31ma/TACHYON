import type { PlacedOrder } from '/models/PlacedOrder.ts';

@Service
export default class CheckoutService {
  @Needs
  view!: HTMLElement;
  @Needs
  lifecycle!: AbortSignal;
  applicationEventPublisher!: ApplicationEventPublisher;
  placed = 0;

  @OnStart
  start(): void {
    this.view.dataset.ready = 'true';
    this.place();
  }

  place(): void {
    this.placed += 1;
    const order: PlacedOrder = { status: 'paid', total: 25 };
    this.applicationEventPublisher.publishEvent(order, true);
  }

  @OnStop
  stop(): void { this.placed = 0; }
}
