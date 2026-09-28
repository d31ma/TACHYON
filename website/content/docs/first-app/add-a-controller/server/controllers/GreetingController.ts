import type { Greeting } from '/models/Greeting.ts'

@RequestRoute("/api/greeting")
@Public
class GreetingController {
  @GetRoute
  getGreeting(): Greeting {
    return { message: 'Hello from Yon' }
  }
}
