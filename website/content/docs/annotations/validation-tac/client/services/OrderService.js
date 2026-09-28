@Service
export default class OrderService {
  status = 'ready';
  /**
   * @param {OrderInput} order
   * @returns {OrderInput}
   */
  accept(@Check order) { return order; }
}
