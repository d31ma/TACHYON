import type { Greeting } from '/models/Greeting.ts'

@HTTPClient
export default class GreetingClient {
  @GetRequest("/api/greeting")
  async getGreeting(): Promise<Greeting | void> {}
}
