@Service
class OrdersService {
  behaviors!: BehaviorContext;
  private static readonly FIRST_WORKING_ATTEMPT = 3;
  private attempts = 0;

  @OnStart
  async open() {
    const signal = new AbortController().signal;
    this.behaviors = await Behaviors.create(OrdersSetup, { partition: 'orders', signal });
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
    if (this.attempts < OrdersService.FIRST_WORKING_ATTEMPT) throw new Error('the payment gateway is unavailable');
  }
}
