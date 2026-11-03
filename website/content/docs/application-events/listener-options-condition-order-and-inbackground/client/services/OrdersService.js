/** @import { PlacedOrder } from '/models/PlacedOrder.ts' */
/** @import { SettledOrder } from '/models/SettledOrder.ts' */
@Service
export default class OrdersService {
  /** @type {ApplicationEventPublisher} */
  applicationEventPublisher;
  /** @type {string[]} */
  log = [];
  settled = 0;
  audited = 0;

  /**
   * @param {Event} _event
   * @param {string} total
   */
  place(_event, total) {
    /** @type {PlacedOrder} */
    const order = { status: 'paid', total: Number(total) };
    this.applicationEventPublisher.publishEvent(order);
  }

  /** @param {PlacedOrder} order */
  @OnEvent({"id":"orders-log"})
  @Order(-10)
  record(order) { this.log = [...this.log, order.status]; }

  /**
   * @param {PlacedOrder} order
   * @returns {SettledOrder}
   */
  @OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})
  settleLargeOrder(order) {
    this.settled += 1;
    return { total: order.total };
  }

  /** @param {SettledOrder} settled */
  @OnEvent
  @InBackground
  async audit(settled) { this.audited = settled.total; }
}
