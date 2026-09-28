@Service
export default class BasketService {
  _total = 1250

  /** @returns {string} */
  totalLabel() {
    return PriceUtils.label(this._total)
  }
}
