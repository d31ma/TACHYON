export class OrderService {
  /**
   * @param {string} item
   * @returns {{ placed: string }}
   */
  static place(item) {
    return { placed: item }
  }
}
