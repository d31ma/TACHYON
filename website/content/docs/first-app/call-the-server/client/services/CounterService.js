/** @import { Greeting } from '/models/Greeting.ts' */
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
    this.message = this._messageOf(greeting)
  }

  /**
   * @param {Greeting | void} greeting
   * @returns {string}
   */
  _messageOf(greeting) {
    return greeting ? greeting.message : ''
  }
}
