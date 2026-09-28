/** @import { PlacedOrder } from '/models/PlacedOrder.ts' */
/** @import { SettledOrder } from '/models/SettledOrder.ts' */
@Service
export default class AuditService {
  /** @type {string[]} */
  _log = [];
  settled = 0;
  _audited = 0;

  /** @param {PlacedOrder} order */
  @OnEvent({"id":"audit-log","replay":true})
  @Order(-10)
  record(order) { this._log = [...this._log, order.status]; }

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
  async audit(settled) { this._audited = settled.total; }
}
