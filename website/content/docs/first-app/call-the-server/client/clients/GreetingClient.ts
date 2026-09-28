import type { Greeting } from '/models/Greeting.ts'

@HTTPClient
class GreetingClient {
  @GetRequest("/api/greeting")
  async getGreeting(): Promise<Greeting> {}
}
