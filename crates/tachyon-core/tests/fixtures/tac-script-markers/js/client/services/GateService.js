@Service
export default class GateService {
  /** @type {AbortSignal} */
  @Needs
  lifecycle;
  /** @type {BehaviorContext} */
  behaviors;
  /** @type {string[]} */
  steps = [];
  tries = 0;

  @OnStart
  async open() {
    this.behaviors = await Behaviors.create(OrdersSetup, { partition: 'gate', signal: this.lifecycle });
  }

  /**
   * @param {string} name
   * @returns {Promise<string>}
   */
  @MaxAtOnce({"limit":1,"policy":"reject"})
  async once(name) {
    await GateService._pause();
    return name;
  }

  /**
   * @param {string} name
   * @returns {Promise<string>}
   */
  @MaxAtOnce({"limit":1,"policy":"block"})
  async queued(name) {
    this.steps = [...this.steps, `start ${name}`];
    await GateService._pause();
    this.steps = [...this.steps, `end ${name}`];
    return name;
  }

  /** @returns {Promise<number>} */
  @Retry({"maxRetries":2,"delayMs":1,"multiplier":1,"maxDelayMs":5,"jitterMs":0})
  async unreachable() {
    this.tries += 1;
    throw Error(`try ${this.tries} failed`);
  }

  /** @returns {Promise<void>} */
  static _pause() {
    return new Promise((resolve) => setTimeout(resolve, 20));
  }
}
