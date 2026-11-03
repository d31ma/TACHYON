@Service
export default class GateService {
  @Needs
  lifecycle!: AbortSignal;
  behaviors!: BehaviorContext;
  steps: string[] = [];
  tries = 0;

  @OnStart
  async open() {
    this.behaviors = await Behaviors.create(OrdersSetup, { partition: 'gate', signal: this.lifecycle });
  }

  @MaxAtOnce({"limit":1,"policy":"reject"})
  async once(name: string): Promise<string> {
    await GateService.pause();
    return name;
  }

  @MaxAtOnce({"limit":1,"policy":"block"})
  async queued(name: string): Promise<string> {
    this.steps = [...this.steps, `start ${name}`];
    await GateService.pause();
    this.steps = [...this.steps, `end ${name}`];
    return name;
  }

  @Retry({"maxRetries":2,"delayMs":1,"multiplier":1,"maxDelayMs":5,"jitterMs":0})
  async unreachable(): Promise<number> {
    this.tries += 1;
    throw Error(`try ${this.tries} failed`);
  }

  private static pause(): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, 20));
  }
}
