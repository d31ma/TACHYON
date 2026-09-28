@Test(HelloTestSetup, {"partition":"tests"})
export default class CounterServiceTest {
  @Fake
  greetingClient!: GreetingClient;

  async testIncrementAddsOne(): Promise<void> {
    this.greetingClient = { getGreeting: async () => ({ message: 'Hi' }) } as GreetingClient
    const counter = this.construct(CounterService)
    counter.increment()
    if (counter.count !== 1) throw Error(`count is ${counter.count}`)
  }

  async testAskServerSavesTheMessage(): Promise<void> {
    this.greetingClient = { getGreeting: async () => ({ message: 'Hi' }) } as GreetingClient
    const counter = this.construct(CounterService)
    await counter.askServer()
    if (counter.message !== 'Hi') throw Error(`message is ${counter.message}`)
  }
}
