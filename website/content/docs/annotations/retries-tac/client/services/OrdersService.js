@Service
export default class OrdersService {
  /** @type {AbortSignal} */
  @Needs
  lifecycle;
  /** @type {BehaviorContext} */
  behaviors;
  static _FIRST_WORKING_ATTEMPT = 3;
  attempts = 0;

  @OnStart
  async open() {
    this.behaviors = await Behaviors.create(OrdersSetup, { partition: 'orders', signal: this.lifecycle });
  }

  /**
   * @param {string} id
   * @returns {Promise<{ id: string, attempts: number }>}
   */
  @Retry({"maxRetries":3,"delayMs":10,"multiplier":2,"maxDelayMs":200,"jitterMs":5})
  async submit(id) {
    this.attempts += 1;
    this.#failFirstTwoAttempts();
    return { id, attempts: this.attempts };
  }

  /**
   * @param {string} id
   * @returns {Promise<{ id: string, attempts: number }>}
   */
  @MaxAtOnce({"limit":2,"policy":"block"})
  async report(id) { return { id, attempts: this.attempts }; }

  #failFirstTwoAttempts() {
    if (this.attempts < OrdersService._FIRST_WORKING_ATTEMPT) throw Error('the checkout API is unavailable');
  }
}
