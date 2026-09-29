/** @import { Greeting } from '/models/Greeting.ts' */
@HTTPClient
export default class GreetingClient {
  /** @returns {Promise<Greeting | void>} */
  @GetRequest("/api/greeting")
  async getGreeting() {}
}
