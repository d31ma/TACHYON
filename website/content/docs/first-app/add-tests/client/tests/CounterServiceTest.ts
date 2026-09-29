@Test(HelloTestSetup, {"partition":"tests"})
export default class CounterServiceTest {
  @Fake
  greetingClient!: GreetingClient;

  async testIncrementAddsOne(): Promise<void> {
    this.greetingClient = { getGreeting: async () => ({ message: 'Hi' }) } as GreetingClient
    const counter = this.construct(CounterService)
    counter.increment()
    Assert.equals(1, counter.count)
  }

  async testAskServerSavesTheMessage(): Promise<void> {
    this.greetingClient = { getGreeting: async () => ({ message: 'Hi' }) } as GreetingClient
    const counter = this.construct(CounterService)
    await counter.askServer()
    Assert.equals('Hi', counter.message)
  }
}
