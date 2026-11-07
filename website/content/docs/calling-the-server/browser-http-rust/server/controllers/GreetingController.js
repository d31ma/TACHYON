/** @import { Greeting } from '/models/Greeting.ts' */
@RequestRoute("/api/greeting")
@Public
export default class GreetingController {
  /** @returns {Promise<Greeting>} */
  @GetRoute
  async getGreeting() {
    await new Promise(resolve => setTimeout(resolve, 150));
    return { message: 'Hello from Yon' };
  }
}
