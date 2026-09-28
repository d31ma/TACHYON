import type { PlacedOrder } from '/models/PlacedOrder';
import type { SettledOrder } from '/models/SettledOrder';

@Service
export default class AuditService {
  private log: string[] = [];
  settled = 0;
  private audited = 0;

  @OnEvent({"id":"audit-log","replay":true})
  @Order(-10)
  record(order: PlacedOrder): void { this.log = [...this.log, order.status]; }

  @OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})
  @Order(10)
  settle(order: PlacedOrder): SettledOrder { return { total: order.total }; }

  @OnEvent
  saveSettled(settled: SettledOrder): void { this.settled = settled.total; }

  @OnEvent
  @InBackground
  async audit(settled: SettledOrder): Promise<void> { this.audited = settled.total; }
}
