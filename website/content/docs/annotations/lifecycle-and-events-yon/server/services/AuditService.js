/** @import { PlacedOrder } from '/models/PlacedOrder.ts' */
/** @import { SettledOrder } from '/models/SettledOrder.ts' */
@Service
class AuditService {
  /** @type {string[]} */
  log = [];
  settled = 0;
  audited = 0;

  /** @param {PlacedOrder} order */
  @OnEvent({"id":"audit-log"})
  @Order(-10)
  record(order) { this.log = [...this.log, order.status]; }

  /**
   * @param {PlacedOrder} order
   * @returns {SettledOrder}
   */
  @OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})
  @Order(10)
  settle(order) { return { total: order.total }; }

  /** @param {SettledOrder} settled */
  @OnEvent
  saveSettled(settled) { this.settled = settled.total; }

  /** @param {SettledOrder} settled */
  @OnEvent
  @InBackground
  async audit(settled) { this.audited = settled.total; }
}
