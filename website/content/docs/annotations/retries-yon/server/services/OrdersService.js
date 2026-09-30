@Service
export default class OrdersService {
  /** @type {BehaviorContext} */
  behaviors;
  static _FIRST_WORKING_ATTEMPT = 3;
  _attempts = 0;

  @OnStart
  async open() {
    const signal = new AbortController().signal;
    this.behaviors = await Behaviors.create(OrdersSetup, { partition: 'orders', signal });
  }

  /**
   * @param {string} id
   * @returns {Promise<{ id: string, attempts: number }>}
   */
  @Retry({"maxRetries":3,"delayMs":10,"multiplier":2,"maxDelayMs":200,"jitterMs":5})
  async submit(id) {
    this._attempts += 1;
    this._failFirstTwoAttempts();
    return { id, attempts: this._attempts };
  }

  /**
   * @param {string} id
   * @returns {Promise<{ id: string, attempts: number }>}
   */
  @MaxAtOnce({"limit":2,"policy":"block"})
  async report(id) { return { id, attempts: this._attempts }; }

  _failFirstTwoAttempts() {
    if (this._attempts < OrdersService._FIRST_WORKING_ATTEMPT) throw new Error('the payment gateway is unavailable');
  }
}
