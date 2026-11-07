import type { Greeting } from '/models/Greeting.ts'

@RequestRoute("/api/greeting")
@Public
export default class GreetingController {
  @GetRoute
  getGreeting(): Greeting {
    return { message: 'Hello from Yon' }
  }
}
