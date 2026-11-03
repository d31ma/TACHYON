import type { PlacedOrder } from '/models/PlacedOrder.ts';
@Service
export default class CheckoutService {
  @Needs
  applicationEventPublisher!: ApplicationEventPublisher;

  @OnStart
  start(): void {
    const order: PlacedOrder = { status: 'paid', total: 25 };
    this.applicationEventPublisher.publishEvent(order);
  }

  @OnStop
  stop(): void {}
}
