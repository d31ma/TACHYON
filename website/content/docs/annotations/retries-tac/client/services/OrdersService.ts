@Service
export default class OrdersService {
  @Needs
  lifecycle!: AbortSignal;
  behaviors!: BehaviorContext;
  private static readonly FIRST_WORKING_ATTEMPT = 3;
  attempts = 0;

  @OnStart
  async open() {
    this.behaviors = await Behaviors.create(OrdersSetup, { partition: 'orders', signal: this.lifecycle });
  }

  @Retry({"maxRetries":3,"delayMs":10,"multiplier":2,"maxDelayMs":200,"jitterMs":5})
  async submit(id: string) {
    this.attempts += 1;
    this.failFirstTwoAttempts();
    return { id, attempts: this.attempts };
  }

  @MaxAtOnce({"limit":2,"policy":"block"})
  async report(id: string) { return { id, attempts: this.attempts }; }

  private failFirstTwoAttempts() {
    if (this.attempts < OrdersService.FIRST_WORKING_ATTEMPT) throw Error('the checkout API is unavailable');
  }
}
