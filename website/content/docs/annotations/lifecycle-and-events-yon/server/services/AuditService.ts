import type { PlacedOrder } from '/models/PlacedOrder';
import type { SettledOrder } from '/models/SettledOrder';
@Service
class AuditService {
  log: string[] = [];
  settled = 0;
  audited = 0;

  @OnEvent({"id":"audit-log"})
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
