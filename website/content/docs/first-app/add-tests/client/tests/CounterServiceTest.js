@Test(HelloTestSetup, {"partition":"tests"})
export default class CounterServiceTest {
  /** @type {GreetingClient} */
  @Fake
  greetingClient;

  async testIncrementAddsOne() {
    this.greetingClient = { getGreeting: async () => ({ message: 'Hi' }) }
    const counter = this.construct(CounterService)
    counter.increment()
    Assert.equals(1, counter.count)
  }

  async testAskServerSavesTheMessage() {
    this.greetingClient = { getGreeting: async () => ({ message: 'Hi' }) }
    const counter = this.construct(CounterService)
    await counter.askServer()
    Assert.equals('Hi', counter.message)
  }
}
