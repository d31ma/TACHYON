@Service
export default class OrderService {
  /**
   * @param {OrderInput} order
   * @returns {OrderInput}
   */
  accept(@Check order) {
    return order;
  }
}
