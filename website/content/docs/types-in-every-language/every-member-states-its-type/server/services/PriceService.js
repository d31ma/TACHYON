@Service
export class PriceService {
  static CURRENCY = 'EUR'

  /**
   * @param {number} amount
   * @param {number} count
   * @returns {Price}
   */
  priceFor(amount, count) {
    return { amount: amount * count, currency: PriceService.CURRENCY }
  }
}
