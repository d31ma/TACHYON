/** @import { Greeting } from '/models/Greeting.ts' */
@RequestRoute("/api/greeting")
@Public
export default class GreetingController {
  /** @returns {Greeting} */
  @GetRoute
  getGreeting() {
    return { message: 'Hello from Yon' }
  }
}
