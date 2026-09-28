@Service
export default class CounterService {
  /** @type {GreetingClient} */
  @Needs
  greetingClient;
  count = 0
  message = ''

  increment() {
    this.count += 1
  }

  async askServer() {
    const greeting = await this.greetingClient.getGreeting()
    this.message = greeting.message
  }
}
