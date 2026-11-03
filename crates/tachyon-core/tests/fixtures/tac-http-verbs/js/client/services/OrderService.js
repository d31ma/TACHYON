@Service
export default class OrderService {
  /** @type {OrdersClient} */
  @Needs
  ordersClient;

  /** @returns {Promise<SentRequest>} */
  async replace() {
    return this.ordersClient.replace('a-7', { item: 'hammer' }, 'north');
  }

  /** @returns {Promise<SentRequest>} */
  async change() {
    return this.ordersClient.change('a-7', { item: 'nails' }, 'south');
  }

  /** @returns {Promise<SentRequest>} */
  async remove() {
    return this.ordersClient.remove('a-7', 'duplicate');
  }
}
