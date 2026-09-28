import type { PlacedOrder } from '/models/PlacedOrder';
import type { SettledOrder } from '/models/SettledOrder';

@Service
export default class OrdersService {
  applicationEventPublisher!: ApplicationEventPublisher;
  log: string[] = [];
  settled = 0;
  audited = 0;

  place(_event: Event, total: string) {
    const order: PlacedOrder = { status: 'paid', total: Number(total) };
    this.applicationEventPublisher.publishEvent(order);
  }

  @OnEvent({"id":"orders-log"})
  @Order(-10)
  record(order: PlacedOrder): void { this.log = [...this.log, order.status]; }

  @OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})
  settleLargeOrder(order: PlacedOrder): SettledOrder {
    this.settled += 1;
    return { total: order.total };
  }

  @OnEvent
  @InBackground
  async audit(settled: SettledOrder): Promise<void> { this.audited = settled.total; }
}
